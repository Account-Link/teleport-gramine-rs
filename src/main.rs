use std::{net::SocketAddr, path::PathBuf};

use alloy::signers::local::{coins_bip39::English, MnemonicBuilder};
use db::{create_tables, open_connection};
use endpoints::{callback, hello_world, mint, new_user, SharedState};
use listener::subscribe_to_events;
use tower_http::cors::CorsLayer;
mod db;
mod endpoints;
mod listener;
mod oai;
mod twitter;
use axum_server::tls_rustls::RustlsConfig;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();
    dotenv::from_filename("/teleport.env").ok();

    let db_url = std::env::var("DB_URL").expect("DB_URL not set");
    let ws_rpc_url = std::env::var("WS_RPC_URL").expect("WS_RPC_URL not set");
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL not set");
    let mnemonic = std::env::var("NFT_MINTER_MNEMONIC").expect("NFT_MINTER_MNEMONIC not set");

    let signer = MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .index(0)
        .unwrap()
        .build()
        .unwrap();

    let mut connection = open_connection(db_url.clone()).expect("Failed to open database");
    create_tables(&mut connection).expect("Failed to create tables");

    let shared_state = SharedState {
        db_url: db_url.clone(),
        wallet: signer.into(),
        rpc_url,
    };
    let app = axum::Router::new()
        .route("/new", axum::routing::get(new_user))
        .route("/callback", axum::routing::get(callback))
        .route("/mint", axum::routing::get(mint))
        .route("/", axum::routing::get(hello_world))
        .layer(CorsLayer::very_permissive())
        .with_state(shared_state);
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
    subscribe_to_events(&mut connection, ws_rpc_url)
        .await
        .unwrap();
}
