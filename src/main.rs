use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use alloy::signers::local::{coins_bip39::English, MnemonicBuilder};
use axum_server::tls_rustls::RustlsConfig;
use endpoints::{callback, hello_world, mint, new_user, redeem, SharedState};
use tokio::sync::Mutex;
use tokio::fs;
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

    let db = db::in_memory::InMemoryDB::new();
    let db = Arc::new(Mutex::new(db));
    let shared_state = SharedState {
        db: db.clone(),
        wallet: signer.into(),
        rpc_url,
    };

    let eph = fs::read("/tmp/key.pem").await.expect("gramine ratls rootCA.key not found");
    let gram_crt = fs::read("/tmp/crt.pem").await.expect("gramine ratls rootCA.crt not found");

    let remove_str = "TRUSTED C";

    let mut gram_crt_print = fs::read_to_string("/tmp/crt.pem").await.expect("gramine ratls rootCA.crt not found");
    let mut remove_offset = gram_crt_print.find(remove_str).unwrap_or(gram_crt_print.len());
    gram_crt_print.replace_range(remove_offset..remove_offset + remove_str.len(), "C");
    remove_offset = gram_crt_print.find(remove_str).unwrap_or(gram_crt_print.len());
    gram_crt_print.replace_range(remove_offset..remove_offset + remove_str.len(), "C");

    
    let app = axum::Router::new()
        .route("/new", axum::routing::get(new_user))
        .route("/callback", axum::routing::get(callback))
        .route("/mint", axum::routing::get(mint))
        .route("/redeem", axum::routing::get(redeem))
        .route("/", axum::routing::get(hello_world))
        .layer(CorsLayer::very_permissive())
        .with_state(shared_state);
    let config = RustlsConfig::from_pem(
        gram_crt_print.as_bytes().to_vec(), 
        eph
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
