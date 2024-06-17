use std::sync::Arc;

use alloy::providers::network::EthereumWallet;
use axum::{
    extract::{Query, State},
    response::Redirect,
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    db::{User, UserDB},
    listener::mint_nft,
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
}

#[derive(Serialize)]
pub struct MintResponse {
    pub hash: String,
}

#[derive(Clone)]
pub struct SharedState<A: UserDB> {
    pub db: Arc<Mutex<A>>,
    pub rpc_url: String,
    pub wallet: EthereumWallet,
}

pub async fn new_user<A: UserDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<NewUserQuery>,
) -> Redirect {
    let teleport_id = query.teleport_id;
    // let mut connection =
    //     open_connection(shared_state.db_url.clone()).expect("Failed to open database");
    let (oauth_token, oauth_token_secret) = request_oauth_token(teleport_id.clone())
        .await
        .expect("Failed to request oauth token");
    let user = User {
        x_id: None,
        access_token: oauth_token.clone(),
        access_secret: oauth_token_secret,
        address: query.address,
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

pub async fn callback<A: UserDB>(
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
    // let mut connection =
    //     open_connection(shared_state.db_url.clone()).expect("Failed to open database");

    // let oauth_user = get_oauth_user_by_teleport_id(&mut connection, teleport_id.clone())
    //     .await
    //     .expect("Failed to get oauth tokens");
    assert_eq!(oauth_token, oauth_user.access_token);

    let (access_token, access_secret) =
        authorize_token(oauth_token, oauth_user.access_secret, oauth_verifier)
            .await
            .unwrap();
    let x_info = get_user_x_info(access_token.clone(), access_secret.clone()).await;

    let user = User {
        x_id: Some(x_info.id.clone()),
        access_token,
        access_secret,
        address: oauth_user.address,
    };

    // add_user(&mut connection, user).await.unwrap();
    db.add_user(teleport_id.clone(), user)
        .await
        .expect("Failed to add user to database");
    drop(db);

    let encoded_x_info =
        serde_urlencoded::to_string(&x_info).expect("Failed to encode x_info as query params");
    let url_with_params = format!(
        "http://localhost:4000/create?success=true&{}",
        encoded_x_info
    );

    Redirect::temporary(&url_with_params)
}

pub async fn mint<A: UserDB>(
    State(shared_state): State<SharedState<A>>,
    Query(query): Query<MintQuery>,
) -> Json<MintResponse> {
    // let mut connection =
    //     open_connection(shared_state.db_url.clone()).expect("Failed to open database");
    // let user = get_user_by_teleport_id(&mut connection, query.teleport_id)
    //     .await
    //     .expect("Failed to get user by teleport_id");
    let db = shared_state.db.lock().await;
    let user = db
        .get_user_by_teleport_id(query.teleport_id.clone())
        .await
        .expect("Failed to get user by teleport_id");
    drop(db);

    let tx_hash = mint_nft(
        shared_state.wallet,
        shared_state.rpc_url,
        user.address,
        user.x_id.expect("User x_id not set"),
        query.policy,
    )
    .await
    .expect("Failed to mint NFT");
    Json(MintResponse { hash: tx_hash })
}

pub async fn hello_world() -> &'static str {
    log::info!("Hello, World!");
    "Hello, World!"
}
