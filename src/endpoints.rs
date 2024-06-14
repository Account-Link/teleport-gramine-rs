use alloy::providers::network::EthereumWallet;
use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;

use crate::{
    db::{
        add_oauth_user, add_user, get_oauth_user_by_teleport_id, get_user_by_teleport_id,
        open_connection, OAuthUser, User,
    },
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

#[derive(Clone)]
pub struct SharedState {
    pub db_url: String,
    pub rpc_url: String,
    pub wallet: EthereumWallet,
}

pub async fn new_user(
    State(shared_state): State<SharedState>,
    Query(query): Query<NewUserQuery>,
) -> Redirect {
    let teleport_id = query.teleport_id;
    let mut connection =
        open_connection(shared_state.db_url.clone()).expect("Failed to open database");
    let (oauth_token, oauth_token_secret) = request_oauth_token(teleport_id.clone())
        .await
        .expect("Failed to request oauth token");
    let user = OAuthUser {
        teleport_id: teleport_id.clone(),
        oauth_token: oauth_token.clone(),
        oauth_token_secret,
        address: query.address,
    };
    add_oauth_user(&mut connection, user)
        .await
        .expect("Failed to add oauth tokens to database");

    let url = format!(
        "https://api.twitter.com/oauth/authenticate?oauth_token={}",
        oauth_token
    );

    Redirect::temporary(&url)
}

pub async fn callback(
    State(shared_state): State<SharedState>,
    Query(query): Query<CallbackQuery>,
) -> Redirect {
    let oauth_token = query.oauth_token;
    let oauth_verifier = query.oauth_verifier;
    let teleport_id = query.teleport_id;
    let mut connection =
        open_connection(shared_state.db_url.clone()).expect("Failed to open database");

    let oauth_user = get_oauth_user_by_teleport_id(&mut connection, teleport_id.clone())
        .await
        .expect("Failed to get oauth tokens");
    assert_eq!(oauth_token, oauth_user.oauth_token);

    let (access_token, access_secret) =
        authorize_token(oauth_token, oauth_user.oauth_token_secret, oauth_verifier)
            .await
            .unwrap();
    let x_info = get_user_x_info(access_token.clone(), access_secret.clone()).await;

    let user = User {
        x_id: x_info.id.clone(),
        teleport_id,
        access_token,
        access_secret,
        address: oauth_user.address,
    };

    add_user(&mut connection, user).await.unwrap();

    let encoded_x_info =
        serde_urlencoded::to_string(&x_info).expect("Failed to encode x_info as query params");
    let url_with_params = format!(
        "http://localhost:4000/create?success=true&{}",
        encoded_x_info
    );

    Redirect::temporary(&url_with_params)
}

pub async fn mint(
    State(shared_state): State<SharedState>,
    Query(query): Query<MintQuery>,
) -> String {
    let mut connection =
        open_connection(shared_state.db_url.clone()).expect("Failed to open database");
    let user = get_user_by_teleport_id(&mut connection, query.teleport_id)
        .await
        .expect("Failed to get user by teleport_id");

    mint_nft(
        shared_state.wallet,
        shared_state.rpc_url,
        user.address,
        user.x_id,
        query.policy,
    )
    .await
    .expect("Failed to mint NFT")
}
