use alloy::primitives::Address;
use std::{str::FromStr, sync::Arc};

use axum::{
    extract::{Query, State},
    response::Redirect,
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    actions::{
        nft::{mint_nft, redeem_nft},
        wallet::WalletProvider,
    },
    db::{PendingNFT, TeleportDB, User},
    oai,
    twitter::{authorize_token, get_user_x_info, request_oauth_token},
};

#[derive(Deserialize)]
pub struct NewUserQuery {
    address: String,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    oauth_token: String,
    oauth_verifier: String,
    address: String,
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
    pub app_url: String,
    pub tee_url: String,
}

pub async fn new_user<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<NewUserQuery>,
) -> Redirect {
    let address = query.address;

    let db_lock = shared_state.db.lock().await;
    let existing_user = db_lock.get_user_by_address(address.clone()).await.ok();
    if let Some(user) = existing_user {
        if user.x_id.is_some() {
            let x_info = get_user_x_info(user.access_token, user.access_secret).await;
            let encoded_x_info = serde_urlencoded::to_string(&x_info)
                .expect("Failed to encode x_info as query params");
            let url_with_params = format!(
                "{}/create?already_created=true&success=true&{}",
                shared_state.app_url, encoded_x_info
            );
            return Redirect::temporary(&url_with_params);
        }
    }
    drop(db_lock);

    let (oauth_token, oauth_token_secret) =
        request_oauth_token(address.clone(), shared_state.tee_url)
            .await
            .expect("Failed to request oauth token");
    let user =
        User { x_id: None, access_token: oauth_token.clone(), access_secret: oauth_token_secret };
    let mut db = shared_state.db.lock().await;
    db.add_user(address.clone(), user).await.expect("Failed to add oauth tokens to database");
    drop(db);

    let url = format!("https://api.twitter.com/oauth/authenticate?oauth_token={}", oauth_token);

    Redirect::temporary(&url)
}

pub async fn callback<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<CallbackQuery>,
) -> Redirect {
    let oauth_token = query.oauth_token;
    let oauth_verifier = query.oauth_verifier;
    let address = query.address;

    let mut db = shared_state.db.lock().await;
    let oauth_user =
        db.get_user_by_address(address.clone()).await.expect("Failed to get oauth tokens");
    assert_eq!(oauth_token, oauth_user.access_token);

    let (access_token, access_secret) =
        authorize_token(oauth_token, oauth_user.access_secret, oauth_verifier).await.unwrap();
    let x_info = get_user_x_info(access_token.clone(), access_secret.clone()).await;
    let user = User { x_id: Some(x_info.id.clone()), access_token, access_secret };
    db.add_user(address, user.clone()).await.expect("Failed to add user to database");
    drop(db);

    let encoded_x_info =
        serde_urlencoded::to_string(&x_info).expect("Failed to encode x_info as query params");
    let url_with_params =
        format!("{}/create?success=true&{}", shared_state.app_url, encoded_x_info);

    Redirect::temporary(&url_with_params)
}

pub async fn mint<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<MintQuery>,
) -> Json<TxHashResponse> {
    let db = shared_state.db.lock().await;
    let user =
        db.get_user_by_address(query.address.clone()).await.expect("Failed to get user by address");
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
    .await
    .expect("Failed to add pending NFT");
    drop(db);

    Json(TxHashResponse { hash: tx_hash })
}

pub async fn redeem<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Json(query): Json<RedeemQuery>,
) -> Json<TxHashResponse> {
    let db = shared_state.db.lock().await;
    let nft = db.get_nft(query.nft_id.clone()).await.unwrap_or_else(|_| panic!("Failed to get NFT by id {}", query.nft_id.to_string()));
    drop(db);

    let tx_hash = redeem_nft(shared_state.provider, nft.token_id.clone(), query.content)
        .await
        .expect(format!("Failed to redeem NFT with id {}", nft.token_id).as_str());
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
    let tweet_id = db.get_tweet(query.token_id.clone()).await.expect("Failed to get tweet id");
    drop(db);

    Json(TweetIdResponse { tweet_id })
}

pub async fn hello_world() -> &'static str {
    log::info!("Hello, World!");
    "Hello, World!"
}
