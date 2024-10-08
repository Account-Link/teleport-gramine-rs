use axum::Router;
use tower_http::cors::CorsLayer;

use crate::db::in_memory::InMemoryDB;
use crate::endpoints::{
    approve_mint, callback, check_redeem, cookietest, get_tweet_id, hello_world, mint, redeem,
    register_or_login, SharedState,
};

pub fn create_router(shared_state: SharedState<InMemoryDB>) -> Router {
    Router::new()
        .route("/new", axum::routing::get(register_or_login))
        .route("/approve", axum::routing::get(approve_mint))
        .route("/callback", axum::routing::get(callback))
        .route("/cookietest", axum::routing::get(cookietest))
        .route("/mint", axum::routing::post(mint))
        .route("/redeem", axum::routing::post(redeem))
        .route("/checkRedeem", axum::routing::post(check_redeem))
        .route("/tweetId", axum::routing::get(get_tweet_id))
        .route("/", axum::routing::get(hello_world))
        .layer(CorsLayer::permissive())
        .with_state(shared_state)
}