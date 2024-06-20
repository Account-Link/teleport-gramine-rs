use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::Redirect,
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    actions::{
        nft::{mint_nft, redeem_nft, send_eth},
        wallet::{gen_sk, WalletProvider},
    },
    db::{PendingNFT, TeleportDB, User},
    twitter::{authorize_token, get_user_x_info, request_oauth_token},
};

#[derive(Deserialize)]
pub struct NewUserQuery {
    teleport_id: String,
    address: String,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    oauth_token: String,
    oauth_verifier: String,
    teleport_id: String,
}

#[derive(Deserialize)]
pub struct MintQuery {
    teleport_id: String,
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

#[derive(Deserialize)]
pub struct RedeemQuery {
    nft_id: String,
    content: String,
}

#[derive(Serialize)]
pub struct TxHashResponse {
    pub hash: String,
}

#[derive(Clone)]
pub struct SharedState<A: TeleportDB> {
    pub db: Arc<Mutex<A>>,
    pub rpc_url: String,
    pub provider: WalletProvider,
}

pub async fn new_user<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<NewUserQuery>,
) -> Redirect {
    let teleport_id = query.teleport_id;

    let db_lock = shared_state.db.lock().await;
    let existing_user = db_lock
        .get_user_by_teleport_id(teleport_id.clone())
        .await
        .ok();
    if let Some(user) = existing_user {
        if user.x_id.is_some() {
            let x_info = get_user_x_info(user.access_token, user.access_secret).await;
            let encoded_x_info = serde_urlencoded::to_string(&x_info)
                .expect("Failed to encode x_info as query params");
            let url_with_params = format!(
                "https://teleport.best/create?already_created=true&success=true&{}",
                encoded_x_info
            );
            return Redirect::temporary(&url_with_params);
        }
    }
    drop(db_lock);

    let (oauth_token, oauth_token_secret) = request_oauth_token(teleport_id.clone())
        .await
        .expect("Failed to request oauth token");
    let user = User {
        x_id: None,
        access_token: oauth_token.clone(),
        access_secret: oauth_token_secret,
        embedded_address: query.address,
        sk: None,
    };
    let mut db = shared_state.db.lock().await;
    db.add_user(teleport_id.clone(), user)
        .await
        .expect("Failed to add oauth tokens to database");
    drop(db);

    let url = format!(
        "https://api.twitter.com/oauth/authenticate?oauth_token={}",
        oauth_token
    );

    Redirect::temporary(&url)
}

pub async fn callback<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<CallbackQuery>,
) -> Redirect {
    let oauth_token = query.oauth_token;
    let oauth_verifier = query.oauth_verifier;
    let teleport_id = query.teleport_id;

    let mut db = shared_state.db.lock().await;
    let oauth_user = db
        .get_user_by_teleport_id(teleport_id.clone())
        .await
        .expect("Failed to get oauth tokens");
    assert_eq!(oauth_token, oauth_user.access_token);

    let (access_token, access_secret) =
        authorize_token(oauth_token, oauth_user.access_secret, oauth_verifier)
            .await
            .unwrap();
    let x_info = get_user_x_info(access_token.clone(), access_secret.clone()).await;
    let sk = gen_sk().expect("Failed to generate sk");
    let user = User {
        x_id: Some(x_info.id.clone()),
        access_token,
        access_secret,
        embedded_address: oauth_user.embedded_address,
        sk: Some(sk),
    };
    db.add_user(teleport_id.clone(), user.clone())
        .await
        .expect("Failed to add user to database");
    drop(db);

    //temp: give eoa some eth for gas
    send_eth(shared_state.provider, user.address().unwrap(), "0.03")
        .await
        .expect("Failed to send eth to eoa");

    let encoded_x_info =
        serde_urlencoded::to_string(&x_info).expect("Failed to encode x_info as query params");
    let url_with_params = format!(
        "https://teleport.best/create?success=true&{}",
        encoded_x_info
    );

    Redirect::temporary(&url_with_params)
}

pub async fn mint<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<MintQuery>,
) -> Json<TxHashResponse> {
    let db = shared_state.db.lock().await;
    let user = db
        .get_user_by_teleport_id(query.teleport_id.clone())
        .await
        .expect("Failed to get user by teleport_id");
    drop(db);

    let tx_hash = mint_nft(
        shared_state.provider,
        user.address().expect("User address not set"),
        user.x_id.expect("User x_id not set"),
        query.policy,
    )
    .await
    .expect("Failed to mint NFT");

    let mut db = shared_state.db.lock().await;
    db.add_pending_nft(
        tx_hash.clone(),
        PendingNFT {
            teleport_id: query.teleport_id.clone(),
            nft_id: query.nft_id.clone(),
        },
    )
    .await
    .expect("Failed to add pending NFT");
    drop(db);

    Json(TxHashResponse { hash: tx_hash })
}

pub async fn redeem<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<RedeemQuery>,
) -> Json<TxHashResponse> {
    let db = shared_state.db.lock().await;
    let nft = db
        .get_nft(query.nft_id.clone())
        .await
        .expect("Failed to get NFT by id");
    let user = db
        .get_user_by_teleport_id(nft.teleport_id.clone())
        .await
        .expect("Failed to get user by teleport_id");
    drop(db);

    let tx_hash = redeem_nft(
        user.signer().unwrap().into(),
        shared_state.rpc_url,
        nft.token_id,
        query.content,
    )
    .await
    .expect("Failed to mint NFT");
    Json(TxHashResponse { hash: tx_hash })
}

pub async fn get_tweet_id<A: TeleportDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<TweetIdQuery>,
) -> Json<TweetIdResponse> {
    let db = shared_state.db.lock().await;
    let tweet_id = db
        .get_tweet(query.token_id.clone())
        .await
        .expect("Failed to get tweet id");
    drop(db);

    Json(TweetIdResponse { tweet_id })
}

pub async fn hello_world() -> &'static str {
    log::info!("Hello, World!");
    "Hello, World!"
}
