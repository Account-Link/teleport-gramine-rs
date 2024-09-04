use std::{net::SocketAddr, sync::Arc};

use alloy::{
    providers::ProviderBuilder,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};

use axum_server::tls_rustls::RustlsConfig;
use endpoints::{
    callback, get_ratls_cert, get_tweet_id, hello_world, mint, new_user, redeem, SharedState,
};
use tokio::{fs, sync::Mutex};
use tower_http::cors::CorsLayer;

use crate::{actions::nft::subscribe_to_nft_events, db::TeleportDB, endpoints::check_redeem};

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

    let ws_rpc_url = std::env::var("WS_RPC_URL").expect("WS_RPC_URL not set");
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL not set");
    let mnemonic = std::env::var("NFT_MINTER_MNEMONIC").expect("NFT_MINTER_MNEMONIC not set");
    let tls_key_path = std::env::var("TLS_KEY_PATH").expect("TLS_KEY_PATH not set");
    let tls_cert_path = std::env::var("TLS_CERT_PATH").expect("TLS_CERT_PATH not set");
    let db_path = std::env::var("DB_PATH").expect("DB_PATH not set");
    let app_url = std::env::var("APP_URL").expect("APP_URL not set");
    let tee_url = std::env::var("TEE_URL").expect("TEE_URL not set");

    let signer =
        MnemonicBuilder::<English>::default().phrase(mnemonic).index(0).unwrap().build().unwrap();

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(signer.into())
        .on_http(rpc_url.parse().unwrap());

    let db = if std::path::Path::new(&db_path).exists() {
        let serialized_bytes = fs::read(&db_path).await.expect("Failed to read db file");
        let db = db::in_memory::InMemoryDB::deserialize(&serialized_bytes);
        log::info!("Loaded db from file: {}", db_path);
        db
    } else {
        db::in_memory::InMemoryDB::new()
    };
    let db = Arc::new(Mutex::new(db));
    let shared_state = SharedState { db: db.clone(), provider, app_url, tee_url };

    let eph = fs::read(tls_key_path).await.expect("gramine ratls rootCA.key not found");

    let remove_str = "TRUSTED C";

    let mut gram_crt_print =
        fs::read_to_string(tls_cert_path).await.expect("gramine ratls rootCA.crt not found");
    let mut remove_offset = 0;
    while let Some(index) = gram_crt_print[remove_offset..].find(remove_str) {
        let start = remove_offset + index;
        let end = start + remove_str.len();
        gram_crt_print.replace_range(start..end, "C");
        remove_offset = end;
    }

    let app = axum::Router::new()
        .route("/new", axum::routing::get(new_user))
        .route("/callback", axum::routing::get(callback))
        .route("/mint", axum::routing::get(mint))
        .route("/redeem", axum::routing::post(redeem))
        .route("/checkRedeem", axum::routing::post(check_redeem))
        .route("/tweetId", axum::routing::get(get_tweet_id))
        .route("/attestSgx", axum::routing::get(get_ratls_cert))
        .route("/", axum::routing::get(hello_world))
        .layer(CorsLayer::very_permissive())
        .with_state(shared_state);
    // let config = RustlsConfig::from_pem(gram_crt_print.as_bytes().to_vec(), eph).await.unwrap();
    // let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
        // axum_server::bind_rustls(addr, config).serve(app.into_make_service()).await.unwrap();
    });
    let db_clone = db.clone();
    tokio::spawn(async move {
        subscribe_to_nft_events(db_clone, ws_rpc_url).await.unwrap();
    });
    tokio::signal::ctrl_c().await.expect("failed to listen for event");
    let db = db.lock().await;
    let serialized = db.serialize().await.unwrap();
    let serialized_bytes = serialized.to_vec();
    fs::write(&db_path, serialized_bytes).await.expect("Failed to save serialized data to file");
    log::info!("Saved db to file: {}", db_path);
    log::info!("Shutting down gracefully");
}
