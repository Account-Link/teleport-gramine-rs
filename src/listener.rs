use axum::{extract::Query, response::Redirect};
use rusqlite::Connection;
use serde::Deserialize;

use crate::{
    db::{add_access_tokens, add_oauth_tokens, get_oauth_tokens_by_teleport_id},
    twitter::{authorize_token, get_user_id, request_oauth_token},
};

#[derive(Deserialize)]
pub struct NewUserQuery {
    teleport_id: String,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    oauth_token: String,
    oauth_verifier: String,
    teleport_id: String,
}

pub async fn new_user(Query(query): Query<NewUserQuery>) -> Redirect {
    let teleport_id = query.teleport_id;
    let db_url = std::env::var("DB_URL").expect("DB_URL not set");
    let mut connection = Connection::open(db_url).expect("Failed to open database connection");
    let (oauth_token, oauth_token_secret) = request_oauth_token(teleport_id.clone())
        .await
        .expect("Failed to request oauth token");

    add_oauth_tokens(
        &mut connection,
        teleport_id.clone(),
        oauth_token.clone(),
        oauth_token_secret,
    )
    .await
    .expect("Failed to add oauth tokens to database");

    let url = format!(
        "https://api.twitter.com/oauth/authenticate?oauth_token={}",
        oauth_token
    );

    Redirect::temporary(&url)
}

pub async fn callback(Query(query): Query<CallbackQuery>) -> Redirect {
    let oauth_token = query.oauth_token;
    let oauth_verifier = query.oauth_verifier;
    let teleport_id = query.teleport_id;
    let db_url = std::env::var("DB_URL").expect("DB_URL not set");
    let mut connection =
        Connection::open(db_url.clone()).expect("Failed to open database connection");

    let (oauth_token_from_db, oauth_token_secret) =
        get_oauth_tokens_by_teleport_id(db_url, teleport_id.clone())
            .await
            .expect("Failed to get oauth tokens");
    assert_eq!(oauth_token, oauth_token_from_db);

    let (access_token, access_secret) =
        authorize_token(oauth_token, oauth_token_secret, oauth_verifier)
            .await
            .unwrap();
    let user_id = get_user_id(access_token.clone(), access_secret.clone()).await;

    add_access_tokens(
        &mut connection,
        user_id,
        teleport_id,
        access_token,
        access_secret,
    )
    .await
    .unwrap();

    //todo: change to actual redirect url
    Redirect::temporary("http://localhost:4000/mint?success=true")
}
