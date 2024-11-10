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
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::{
    actions::nft::{get_token_id, NFTAction},
    db::{in_memory::InMemoryDB, AccessTokens, PendingNFT, Session, TeleportDB, User},
    oai,
    templates::{HtmlTemplate, PolicyTemplate},
    twitter::builder::TwitterBuilder,
};

use alloy::signers::Signer;

use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use eyre::OptionExt;

pub const SESSION_ID_COOKIE_NAME: &str = "teleport_session_id";

fn default_str() -> String {
    "none".to_string()
}

#[derive(Deserialize)]
pub struct NewUserQuery {
    address: String,
    event_id: String,
    user_email: String,
    frontend_url: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    oauth_token: String,
    oauth_verifier: String,
    event_id: String,
    user_email: String,
    address: String,
    frontend_url: String,
}

#[derive(Deserialize)]
pub struct MintQuery {
    address: String,
    policy: String,
}

#[derive(Deserialize)]
pub struct MintEventQuery {
    x_id: String,
    event_id: String,
    address: String,
    policy: String,
    user_email: String,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, Serialize)]
struct LumaUserApprovalRequestQuery {
    event_api_id: String,
    guest: LumaUser,
    status: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct LumaUser {
    r#type: String,
    email: String,
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

#[derive(Serialize)]
pub struct TxHashEventIdResponse {
    pub hash: String,
    pub address: String,
    pub user_email: String,
    pub event_id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct LumaEventQuery {
    api_id: String,
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
    pub signer: LocalSigner<SigningKey>,
    pub app_url: String,
    pub tee_url: String,
    pub twitter_builder: TwitterBuilder,
    pub nft_action_sender: mpsc::Sender<(NFTAction, oneshot::Sender<String>)>,
    pub rpc_url: String,
    pub luma_secret: String,
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

    let callback_url = format!(
        "https://{}/callback?address={}&event_id={}&user_email={}&frontend_url={}",
        shared_state.tee_url.clone(),
        address.clone(),
        query.event_id,
        query.user_email,
        query.frontend_url.unwrap_or(shared_state.app_url)
    );

    let oauth_tokens = shared_state
        .twitter_builder
        .request_oauth_token(callback_url)
        .await
        .expect("Failed to request oauth token");

    let mut db = shared_state.db.lock().await;
    // let mut existing_user = db.get_user_by_address(address.clone()).ok().unwrap_or_default();
    // existing_user.oauth_tokens = oauth_tokens.clone().into();
    db.add_oauth(oauth_tokens.token.clone(), oauth_tokens.secret).expect("Failed to add oauth tokens to database");

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

    let mut db = shared_state.db.lock().await;
    // let mut oauth_user =
    //     db.get_user_by_address(address.clone()).expect("Failed to get oauth tokens");
    let secret = db.get_oauth(oauth_token.clone()).expect("no oauth");
    log::info!("oauth_tokens: {:?}", secret);
    // assert_eq!(oauth_token, oauth_user.oauth_tokens.token);

    let oauth_tokens = AccessTokens { token: oauth_token.clone(), secret: secret };
    let token_pair = shared_state
        .twitter_builder
        .authorize_token(
            oauth_tokens.token.clone(),
            oauth_tokens.secret.clone(),
            oauth_verifier,
        )
        .await
        .unwrap();

    let access_tokens: AccessTokens = token_pair.clone().into();
    let twitter_client = shared_state.twitter_builder.with_auth(token_pair);
    let x_info = twitter_client.get_user_info().await.expect("Failed to get user info");

    let session_id = db
        .add_session(Session { x_id: x_info.id.clone() })
        .expect("Failed to add session to database");

    let existing_user = User {
        x_id: Some(x_info.id.clone()),
        oauth_tokens: oauth_tokens.clone(),
        access_tokens: Some(access_tokens.clone()) };
    db.add_user("".to_string(), existing_user)
        .expect("Failed to add oauth tokens to database");

    let mut oauth_user =
    db.get_user_by_x_id(x_info.id.clone()).expect("Failed to get oauth tokens");
    assert_eq!(oauth_token, oauth_user.oauth_tokens.token);

    if oauth_user.x_id.is_none() {
        oauth_user.x_id = Some(x_info.id.clone());
        oauth_user.access_tokens = Some(access_tokens);
        db.add_user("".to_string(), oauth_user.clone()).expect("Failed to add user to database");
        drop(db);
    }

    let url_with_params = format!(
        "/approve?address={}&policy=anything&x_id={}&event_id={}&user_email={}",
        address.clone(),
        x_info.id.clone(),
        query.event_id,
        query.user_email
    );
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
    Json(query): Json<MintEventQuery>,
) -> Result<Json<TxHashEventIdResponse>, StatusCode> {
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
        db.get_user_by_x_id(query.x_id.clone()).expect("Failed to get user by x_id");

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

    let client = shared_state
        .twitter_builder
        .with_auth(user.access_tokens.ok_or_eyre("User has no access tokens").unwrap().into());

    const LUMA_SECRET_HEADER: &str = "x-luma-api-key";
    log::info!("luma_event_id: {}", query.event_id.clone());
    let event_id_query = LumaEventQuery { api_id: query.event_id.clone() };
    let luma_event = reqwest::Client::new()
        .get("https://api.lu.ma/public/v1/event/get")
        .query(&event_id_query)
        .header(LUMA_SECRET_HEADER, shared_state.luma_secret.clone())
        .send()
        .await
        .expect("Failed to get luma event")
        .json::<serde_json::Value>()
        .await
        .expect("Failed to get luma event json response");
    log::info!("luma_event: {}", luma_event);
    let event_pfp_url =
        luma_event["event"]["cover_url"].as_str().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let user_info = client.get_user_info().await.expect("Failed to get user info");

    let username = if user_info.username.starts_with("@") {
        user_info.username
    } else {
        format!("@{}", user_info.username)
    };

    let nft_id = format!("{:032x}", rand::random::<u128>());

    log::info!("event_pfp_url: {}", event_pfp_url.to_string());

    let nft_action = NFTAction::Mint {
        recipient: Address::from_str(&query.address).expect("Failed to parse user address"),
        policy: query.policy,
        x_id: user.x_id.expect("User x_id not set"),
        name: user_info.name,
        username,
        pfp_url: user_info.profile_image_url.replace("_normal", "_400x400"),
        event_pfp_url: event_pfp_url.to_string(),
        nft_id: nft_id.clone(),
    };

    let (sender, tx_hash) = oneshot::channel();

    shared_state.nft_action_sender.send((nft_action, sender)).await.unwrap();
    let tx_hash = tx_hash.await.unwrap();

    //Approve user on luma
    let luma_request_query = LumaUserApprovalRequestQuery {
        event_api_id: query.event_id.clone(),
        guest: LumaUser { r#type: "email".to_string(), email: query.user_email.clone() },
        status: "approved".to_string(),
    };
    let _ = reqwest::Client::new()
        .post("https://api.lu.ma/public/v1/event/update-guest-status")
        .header(LUMA_SECRET_HEADER, shared_state.luma_secret.clone())
        .body(
            serde_json::to_string(&luma_request_query)
                .expect("Failed to serialize luma request query"),
        )
        .send()
        .await
        .expect("Failed to approve user on luma");

    let mut db = shared_state.db.lock().await;
    db.add_pending_nft(tx_hash.clone(), PendingNFT { address: query.address.clone(), nft_id })
        .expect("Failed to add pending NFT");
    drop(db);

    Ok(Json(TxHashEventIdResponse { hash: tx_hash, address: query.address.clone(), user_email: query.user_email.clone(), event_id: query.event_id.clone()  }))
}

pub async fn redeem<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Json(query): Json<RedeemQuery>,
) -> Json<TxHashResponse> {
    let token_id = get_token_id(shared_state.rpc_url, query.nft_id.clone())
        .await
        .unwrap_or_else(|_| panic!("Failed to get NFT by id {}", query.nft_id));
    log::info!("redeem token_id: {}", token_id);

    let nft_action = NFTAction::Redeem { token_id, content: query.content };

    let (sender, tx_hash) = oneshot::channel();

    shared_state.nft_action_sender.send((nft_action, sender)).await.unwrap();
    let tx_hash = tx_hash.await.unwrap();

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
    Query(query): Query<MintEventQuery>,
    jar: CookieJar,
) -> impl IntoResponse {
    if let Some(session_id) = jar.get(SESSION_ID_COOKIE_NAME) {
        let session_id = session_id.value();
        let db = shared_state.db.lock().await;
        let session = db.get_session(session_id.to_string()).expect("Failed to get session");
    } else {
        log::info!("No session found");
        return Err(StatusCode::UNAUTHORIZED);
    }
    let template =
        PolicyTemplate { policy: query.policy, address: query.address, x_id: query.x_id, event_id: query.event_id, user_email: query.user_email };
    Ok(HtmlTemplate(template))
}

pub async fn hello_world() -> &'static str {
    log::info!("Hello, World!");
    "Hello, World!"
}
