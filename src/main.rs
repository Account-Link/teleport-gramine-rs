use std::{net::SocketAddr, sync::Arc};

use alloy::{
    providers::ProviderBuilder,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use axum_server::tls_rustls::RustlsConfig;
use endpoints::{callback, get_tweet_id, hello_world, mint, new_user, redeem, SharedState};
use tokio::{fs, sync::Mutex};
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
    let tls_key_path = std::env::var("TLS_KEY_PATH").expect("TLS_KEY_PATH not set");
    let tls_cert_path = std::env::var("TLS_CERT_PATH").expect("TLS_CERT_PATH not set");

    let signer = MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .index(0)
        .unwrap()
        .build()
        .unwrap();

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(signer.into())
        .on_http(rpc_url.parse().unwrap());

    let db = db::in_memory::InMemoryDB::new();
    let db = Arc::new(Mutex::new(db));
    let shared_state = SharedState {
        db: db.clone(),
        rpc_url,
        provider,
    };

    let eph = fs::read(tls_key_path)
        .await
        .expect("gramine ratls rootCA.key not found");

    let remove_str = "TRUSTED C";

    let mut gram_crt_print = fs::read_to_string(tls_cert_path)
        .await
        .expect("gramine ratls rootCA.crt not found");
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
        .route("/redeem", axum::routing::get(redeem))
        .route("/tweetId", axum::routing::get(get_tweet_id))
        .route("/", axum::routing::get(hello_world))
        .layer(CorsLayer::very_permissive())
        .with_state(shared_state);
    let config = RustlsConfig::from_pem(gram_crt_print.as_bytes().to_vec(), eph)
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
