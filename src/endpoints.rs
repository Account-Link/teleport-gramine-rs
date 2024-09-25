use alloy::{
    primitives::Address,
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
};
use http::HeaderMap;
use std::{str::FromStr, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use rustls::ClientConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::{
    actions::{
        nft::{mint_nft, redeem_nft},
        wallet::WalletProvider,
    },
    db::{in_memory::InMemoryDB, PendingNFT, Session, TeleportDB},
    oai,
    templates::{HtmlTemplate, PolicyTemplate},
    twitter::{builder::TwitterBuilder, get_callback_url},
};

use alloy::signers::Signer;

use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};

pub const SESSION_ID_COOKIE_NAME: &str = "teleport_session_id";

fn default_str() -> String {
    "none".to_string()
}

#[derive(Deserialize)]
pub struct NewUserQuery {
    address: String,
    #[serde(default = "default_str")]
    frontend_nonce: String,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    oauth_token: String,
    oauth_verifier: String,
    address: String,
    frontend_nonce: String,
}

#[derive(Deserialize)]
pub struct MintQuery {
    address: String,
    policy: String,
    nft_id: String,
}

#[derive(Deserialize)]
pub struct TweetIdQuery {
    token_id: String,
}

#[derive(Serialize)]
pub struct TweetIdResponse {
    tweet_id: String,
}

#[derive(Serialize)]
pub struct AttestationResponse {
    cert: String,
}

#[derive(Deserialize)]
pub struct RedeemQuery {
    nft_id: String,
    content: String,
}

#[derive(Serialize)]
pub struct TxHashResponse {
    pub hash: String,
}

#[derive(Deserialize)]
pub struct CheckRedeemQuery {
    pub content: String,
    pub policy: String,
}

#[derive(Serialize)]
pub struct CheckRedeemResponse {
    pub safe: bool,
}

#[derive(Clone)]
pub struct SharedState<A: TeleportDB> {
    pub db: Arc<Mutex<A>>,
    pub provider: WalletProvider,
    pub signer: LocalSigner<SigningKey>,
    pub app_url: String,
    pub tee_url: String,
    pub twitter_builder: TwitterBuilder,
}

pub async fn cookietest<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<()>,
    jar: CookieJar,
) -> (CookieJar, Redirect) {
    (jar.add(Cookie::new(SESSION_ID_COOKIE_NAME, "cookieasdf")), Redirect::temporary("localhost"))
}

pub async fn register_or_login<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<NewUserQuery>,
) -> Redirect {
    let address = query.address;
    let frontend_nonce = query.frontend_nonce;

    let callback_url =
        get_callback_url(shared_state.tee_url.clone(), address.clone(), frontend_nonce);

    let oauth_tokens = shared_state
        .twitter_builder
        .request_oauth_token(callback_url)
        .await
        .expect("Failed to request oauth token");

    let mut db = shared_state.db.lock().await;
    let mut existing_user = db.get_user_by_address(address.clone()).ok().unwrap_or_default();
    existing_user.oauth_tokens = oauth_tokens.clone().into();
    db.add_user(address.clone(), existing_user).expect("Failed to add oauth tokens to database");

    let url =
        format!("https://api.twitter.com/oauth/authenticate?oauth_token={}", oauth_tokens.token);

    Redirect::temporary(&url)
}

pub async fn callback<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<CallbackQuery>,
    jar: CookieJar,
) -> (CookieJar, Redirect) {
    let oauth_token = query.oauth_token;
    let oauth_verifier = query.oauth_verifier;
    let address = query.address;
    let frontend_nonce = query.frontend_nonce;

    let mut db = shared_state.db.lock().await;
    let mut oauth_user =
        db.get_user_by_address(address.clone()).expect("Failed to get oauth tokens");
    assert_eq!(oauth_token, oauth_user.oauth_tokens.token);

    let token_pair = shared_state
        .twitter_builder
        .authorize_token(
            oauth_user.oauth_tokens.token.clone(),
            oauth_user.oauth_tokens.secret.clone(),
            oauth_verifier,
        )
        .await
        .unwrap();

    let access_tokens = token_pair.clone().into();
    let twitter_client = shared_state.twitter_builder.with_auth(token_pair);
    let x_info = twitter_client.get_user_info().await.expect("Failed to get user info");

    let session_id = db
        .add_session(Session { x_id: x_info.id.clone(), address: address.clone() })
        .expect("Failed to add session to database");

    if oauth_user.x_id.is_none() {
        oauth_user.x_id = Some(x_info.id.clone());
        oauth_user.access_tokens = Some(access_tokens);
        db.add_user(address, oauth_user.clone()).expect("Failed to add user to database");
        drop(db);
    }

    let msg = format!("nonce={}&x_id={}", frontend_nonce, x_info.id);
    let sig = shared_state.signer.sign_message(msg.as_bytes()).await.unwrap();

    let encoded_x_info =
        serde_urlencoded::to_string(&x_info).expect("Failed to encode x_info as query params");
    let url_with_params =
        format!("{}/create?sig={:?}&success=true&{}", shared_state.app_url, sig, encoded_x_info);
    (
        jar.add(
            Cookie::build((SESSION_ID_COOKIE_NAME, session_id))
                .secure(true)
                .http_only(false)
                .same_site(SameSite::None),
        ),
        Redirect::temporary(&url_with_params),
    )
}

pub async fn mint(
    jar: CookieJar,
    headers: HeaderMap,
    State(shared_state): State<SharedState<InMemoryDB>>,
    Json(query): Json<MintQuery>,
) -> Result<Json<TxHashResponse>, StatusCode> {
    if let Some(referer) = headers.get("Referer") {
        let referer = referer.to_str().unwrap_or("");
        if !referer.starts_with(&format!("https://{}/approve", shared_state.tee_url)) {
            return Err(StatusCode::FORBIDDEN);
        }
    } else {
        return Err(StatusCode::FORBIDDEN);
    }
    let db = shared_state.db.lock().await;
    let user =
        db.get_user_by_address(query.address.clone()).expect("Failed to get user by address");

    if let Some(session_id) = jar.get(SESSION_ID_COOKIE_NAME) {
        let session_id = session_id.value();
        let session = db.get_session(session_id.to_string()).expect("Failed to getsession");
        if session.x_id != user.x_id.clone().unwrap() {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }
    drop(db);

    let tx_hash = mint_nft(
        shared_state.provider,
        Address::from_str(&query.address).expect("Failed to parse user address"),
        user.x_id.expect("User x_id not set"),
        query.policy,
    )
    .await
    .expect("Failed to mint NFT");

    let mut db = shared_state.db.lock().await;
    db.add_pending_nft(
        tx_hash.clone(),
        PendingNFT { address: query.address, nft_id: query.nft_id.clone() },
    )
    .expect("Failed to add pending NFT");
    drop(db);

    Ok(Json(TxHashResponse { hash: tx_hash }))
}

pub async fn redeem<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Json(query): Json<RedeemQuery>,
) -> Json<TxHashResponse> {
    let db = shared_state.db.lock().await;
    let nft = db
        .get_nft(query.nft_id.clone())
        .unwrap_or_else(|_| panic!("Failed to get NFT by id {}", query.nft_id));
    drop(db);

    let tx_hash = redeem_nft(shared_state.provider, nft.token_id.clone(), query.content)
        .await
        .unwrap_or_else(|_| panic!("Failed to redeem NFT with id {}", nft.token_id));
    Json(TxHashResponse { hash: tx_hash })
}

pub async fn check_redeem<A: TeleportDB>(
    State(_): State<SharedState<A>>,
    Json(query): Json<CheckRedeemQuery>,
) -> Json<CheckRedeemResponse> {
    let safe = oai::is_tweet_safe(&query.content, &query.policy).await;
    Json(CheckRedeemResponse { safe })
}

pub async fn get_tweet_id<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<TweetIdQuery>,
) -> Json<TweetIdResponse> {
    let db = shared_state.db.lock().await;
    let tweet_id = db.get_tweet(query.token_id.clone()).expect("Failed to get tweet id");
    drop(db);

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut config = ClientConfig::new();
    config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    let tls = MakeRustlsConnect::new(config);
    let (client, connection) = tokio_postgres::connect(&database_url, tls).await.unwrap();
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            log::error!("connection error: {}", e);
        }
    });
    let token_id_int: i32 = query.token_id.parse().unwrap();
    client
        .execute(
            "UPDATE \"RedeemedIndex\" SET \"tweetId\" = $1 WHERE \"tokenId\" = $2",
            &[&tweet_id, &token_id_int],
        )
        .await
        .expect("Failed to update tweetId in RedeemedIndex");

    Json(TweetIdResponse { tweet_id })
}

pub async fn approve_mint<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<MintQuery>,
    jar: CookieJar,
) -> impl IntoResponse {
    if let Some(session_id) = jar.get(SESSION_ID_COOKIE_NAME) {
        let session_id = session_id.value();
        let db = shared_state.db.lock().await;
        let session = db.get_session(session_id.to_string()).expect(
            "Failed to get
    session",
        );
        if session.address != query.address {
            log::info!("Session address does not match");
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        log::info!("No session found");
        // return Err(StatusCode::UNAUTHORIZED);
    };
    let template =
        PolicyTemplate { policy: query.policy, address: query.address, nft_id: query.nft_id };
    Ok(HtmlTemplate(template))
}

pub async fn hello_world() -> &'static str {
    log::info!("Hello, World!");
    "Hello, World!"
}
