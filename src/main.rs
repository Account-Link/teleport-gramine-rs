use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use alloy::signers::local::{coins_bip39::English, MnemonicBuilder};
use axum_server::tls_rustls::RustlsConfig;
use endpoints::{callback, hello_world, mint, new_user, redeem, SharedState};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

use crate::actions::nft::subscribe_to_nft_events;

mod actions;
mod db;
mod endpoints;
mod oai;
mod twitter;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();
    dotenv::from_filename("/teleport.env").ok();

    // let db_url = std::env::var("DB_URL").expect("DB_URL not set");
    let ws_rpc_url = std::env::var("WS_RPC_URL").expect("WS_RPC_URL not set");
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL not set");
    let mnemonic = std::env::var("NFT_MINTER_MNEMONIC").expect("NFT_MINTER_MNEMONIC not set");

    let signer = MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .index(0)
        .unwrap()
        .build()
        .unwrap();

    let db = db::in_memory::InMemoryUserDB::new();
    let db = Arc::new(Mutex::new(db));
    let shared_state = SharedState {
        db: db.clone(),
        wallet: signer.into(),
        rpc_url,
    };
    let app = axum::Router::new()
        .route("/new", axum::routing::get(new_user))
        .route("/callback", axum::routing::get(callback))
        .route("/mint", axum::routing::get(mint))
        .route("/redeem", axum::routing::get(redeem))
        .route("/", axum::routing::get(hello_world))
        .layer(CorsLayer::very_permissive())
        .with_state(shared_state);
    // let config = RustlsConfig::from_pem_file(
    //     PathBuf::from("/tmp/cert.pem"),
    //     PathBuf::from("/tmp/key.pem"),
    // )
    // .await
    // .unwrap();
    let config = RustlsConfig::from_pem_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("cert.pem"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("key.pem"),
    )
    .await
    .unwrap();
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    tokio::spawn(async move {
        axum_server::bind_rustls(addr, config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
    subscribe_to_nft_events(db, ws_rpc_url).await.unwrap();
}
