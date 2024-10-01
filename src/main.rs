use std::{net::SocketAddr, path::Path, sync::Arc};

use acme_lib::create_rsa_key;
use alloy::{
    providers::ProviderBuilder,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use tokio::{sync::mpsc, time::Duration};

use axum_server::tls_rustls::RustlsConfig;
use endpoints::{
    approve_mint, callback, cookietest, get_tweet_id, hello_world, mint, redeem, register_or_login,
    SharedState,
};
use openssl::pkey::PKey;
use tokio::{fs, sync::Mutex, time::sleep};
use tower_http::cors::CorsLayer;

use crate::{
    actions::{
        nft::{nft_action_consumer, subscribe_to_nft_events},
        wallet::get_provider,
    },
    cert::create_csr,
    db::TeleportDB,
    endpoints::check_redeem,
    twitter::builder::TwitterBuilder,
};

mod actions;
mod cert;
mod db;
mod endpoints;
mod oai;
mod sgx_attest;
mod templates;
pub mod twitter;

//const PRIVATE_KEY_PATH: &str = "/root/save/private_key.pem";
const PRIVATE_KEY_PATH: &str = "untrustedhost/private_key.pem";
const CERTIFICATE_PATH: &str = "untrustedhost/certificate.pem";
const CSR_PATH: &str = "untrustedhost/request.csr";
const QUOTE_PATH: &str = "untrustedhost/quote.dat";

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();
    dotenv::from_filename("/teleport.env").ok();

    // Published values
    let ws_rpc_url = std::env::var("WS_RPC_URL").expect("WS_RPC_URL not set");
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL not set");
    let tee_url = std::env::var("TEE_URL").expect("TEE_URL not set");

    // Private API values
    let rpc_key = std::env::var("RPC_KEY").expect("RPC_KEY not set");
    let mnemonic = std::env::var("NFT_MINTER_MNEMONIC").expect("NFT_MINTER_MNEMONIC not set");
    let db_path = std::env::var("DB_PATH").expect("DB_PATH not set");
    let app_url = std::env::var("APP_URL").expect("APP_URL not set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let bobu_address = std::env::var("BOBU_ADDRESS").expect("BOBU_ADDRESS not set");
    let app_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let app_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");

    let twitter_builder = TwitterBuilder::new(app_key, app_secret);

    let ws_rpc_url = ws_rpc_url + &rpc_key;
    let rpc_url = rpc_url + &rpc_key;

    let pkey = if std::path::Path::new(PRIVATE_KEY_PATH).exists() {
        let pk_bytes = fs::read(PRIVATE_KEY_PATH).await.expect("Failed to read pk file");
        PKey::private_key_from_pem(pk_bytes.as_slice()).unwrap()
    } else {
        let pk = create_rsa_key(2048);
        let pk_bytes = pk.private_key_to_pem_pkcs8().unwrap();
        fs::write(PRIVATE_KEY_PATH, pk_bytes).await.expect("Failed to write pk to file");
        pk
    };

    let csr = create_csr(&tee_url, &pkey).unwrap();
    let csr_pem_bytes = csr.to_pem().unwrap();
    fs::write(CSR_PATH, csr_pem_bytes).await.expect("Failed to write csr to file");

    let mut pk_bytes = pkey.public_key_to_pem().unwrap();
    let mut csr_pem_bytes = csr.to_pem().unwrap();
    pk_bytes.append(&mut csr_pem_bytes);
    if let Ok(quote) = sgx_attest::sgx_attest(pk_bytes) {
        // handle quote
        log::info!("Writing quote to file: {}", QUOTE_PATH);
        fs::write(QUOTE_PATH, quote).await.expect("Failed to write quote to file");
    }

    let signer =
        MnemonicBuilder::<English>::default().phrase(mnemonic).index(0).unwrap().build().unwrap();

    let provider = get_provider(rpc_url, signer.clone().into());

    let db = if std::path::Path::new(&db_path).exists() {
        let serialized_bytes = fs::read(&db_path).await.expect("Failed to read db file");
        let db = db::in_memory::InMemoryDB::deserialize(&serialized_bytes);
        log::info!("Loaded db from file: {}", db_path);
        db
    } else {
        db::in_memory::InMemoryDB::new()
    };
    let db = Arc::new(Mutex::new(db));
    let (sender, receiver) = mpsc::channel(100);
    let shared_state = SharedState {
        db: db.clone(),
        app_url,
        tee_url,
        signer,
	bobu_address,
        twitter_builder: twitter_builder.clone(),
        nft_action_sender: sender,
    };

    let app = axum::Router::new()
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
        .with_state(shared_state);

    #[cfg(feature = "https")]
    {
        log::info!("Waiting for cert ...");
        while !Path::new(CERTIFICATE_PATH).exists() {
            sleep(Duration::from_secs(1)).await;
        }
        log::info!("Cert found");
        let cert = fs::read(CERTIFICATE_PATH).await.expect("cert not found");
        let config =
            RustlsConfig::from_pem(cert, pkey.private_key_to_pem_pkcs8().unwrap()).await.unwrap();
        let addr = SocketAddr::from(([0, 0, 0, 0], 8001));
        tokio::spawn(async move {
            axum_server::bind_rustls(addr, config).serve(app.into_make_service()).await.unwrap();
        });
    }

    #[cfg(not(feature = "https"))]
    {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
    }

    let db_clone = db.clone();
    tokio::spawn(async move {
        subscribe_to_nft_events(db_clone, twitter_builder, ws_rpc_url, database_url).await.unwrap();
    });
    tokio::spawn(async move {
        nft_action_consumer(receiver, provider).await;
    });
    tokio::signal::ctrl_c().await.expect("failed to listen for event");
    let db = db.lock().await;
    let serialized = db.serialize().unwrap();
    let serialized_bytes = serialized.to_vec();
    fs::write(&db_path, serialized_bytes).await.expect("Failed to save serialized data to file");
    log::info!("Saved db to file: {}", db_path);
    log::info!("Shutting down gracefully");
}
