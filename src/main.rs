use std::{net::SocketAddr, path::Path, sync::Arc};

use acme_lib::create_rsa_key;
use alloy::{
    providers::ProviderBuilder,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use db::in_memory::InMemoryDB;
use tokio::time::Duration;

use axum_server::tls_rustls::RustlsConfig;
use endpoints::{
    approve_mint, callback, cookietest, get_tweet_id, hello_world, mint, redeem, register_or_login,
    SharedState,
};
use openssl::pkey::PKey;
use openssl::x509::X509Req;
use tokio::{fs, sync::Mutex, time::sleep};
use tower_http::cors::CorsLayer;

use crate::{
    actions::nft::subscribe_to_nft_events, cert::create_csr, db::TeleportDB,
    endpoints::check_redeem, twitter::builder::TwitterBuilder,
};

mod actions;
mod cert;
mod config;
mod db;
mod endpoints;
mod oai;
mod sgx_attest;
mod templates;
pub mod twitter;

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = config::Config::new().expect("Failed to load configuration");

    let twitter_builder = TwitterBuilder::new(config.twitter_consumer_key, config.twitter_consumer_secret);

    let ws_rpc_url = format!("{}{}", config.ws_rpc_url, config.rpc_key);
    let rpc_url = format!("{}{}", config.rpc_url, config.rpc_key);

    let pkey = load_or_create_private_key(&config.paths.private_key).await;
    let csr = create_and_save_csr(&config.paths.csr, &config.tee_url, &pkey).await;

    handle_sgx_attestation(&config.paths.quote, &pkey, &csr).await;

    let signer =
        MnemonicBuilder::<English>::default().phrase(config.nft_minter_mnemonic).index(0).unwrap().build().unwrap();

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(signer.clone().into())
        .on_http(rpc_url.parse().unwrap());

    let db = load_or_create_db(&config.db_path).await;
    let db = Arc::new(Mutex::new(db));
    let shared_state = SharedState {
        db: db.clone(),
        provider,
        app_url: config.app_url,
        tee_url: config.tee_url,
        signer,
        twitter_builder: twitter_builder.clone(),
    };

    let app = create_app(shared_state);
    setup_server(app, &pkey, &config.paths.certificate).await;

    // spawn nft event subscription
    let db_clone = db.clone();
    tokio::spawn(async move {
        subscribe_to_nft_events(db_clone, twitter_builder, ws_rpc_url, config.database_url).await.unwrap();
    });

    // handle shutdown
    tokio::signal::ctrl_c().await.expect("failed to listen for event");
    let db = db.lock().await;
    let serialized = db.serialize().unwrap();
    let serialized_bytes = serialized.to_vec();
    fs::write(&config.db_path, serialized_bytes).await.expect("Failed to save serialized data to file");
    log::info!("Saved db to file: {}", config.db_path);
    log::info!("Shutting down gracefully");
}

async fn load_or_create_private_key(private_key_path: &Path) -> PKey<openssl::pkey::Private> {
    if private_key_path.exists() {
        let pk_bytes = fs::read(private_key_path).await.expect("Failed to read pk file");
        PKey::private_key_from_pem(pk_bytes.as_slice()).unwrap()
    } else {
        let pk = create_rsa_key(2048);
        let pk_bytes = pk.private_key_to_pem_pkcs8().unwrap();
        fs::write(private_key_path, pk_bytes).await.expect("Failed to write pk to file");
        pk
    }
}

async fn create_and_save_csr(
    csr_path: &Path,
    tee_url: &str,
    pkey: &PKey<openssl::pkey::Private>,
) -> X509Req {
    let csr = create_csr(tee_url, pkey).unwrap();
    let csr_pem_bytes = csr.to_pem().unwrap();
    fs::write(csr_path, csr_pem_bytes).await.expect("Failed to write csr to file");
    csr
}

async fn load_or_create_db(db_path: &str) -> InMemoryDB {
    let path = std::path::Path::new(db_path);
    if path.exists() {
        let serialized_bytes = fs::read(&path).await.expect("Failed to read db file");
        let db = InMemoryDB::deserialize(&serialized_bytes);
        log::info!("Loaded db from file: {}", db_path);
        db
    } else {
        db::in_memory::InMemoryDB::new()
    }
}

fn create_app(shared_state: SharedState<InMemoryDB>) -> axum::Router {
    axum::Router::new()
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

async fn setup_server(
    app: axum::Router,
    pkey: &PKey<openssl::pkey::Private>,
    certificate_path: &Path,
) {
    #[cfg(feature = "https")]
    {
        log::info!("Waiting for cert ...");
        while !certificate_path.exists() {
            sleep(Duration::from_secs(1)).await;
        }
        log::info!("Cert found");
        let cert = fs::read(certificate_path).await.expect("cert not found");
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
}

async fn handle_sgx_attestation(quote_path: &Path, pkey: &PKey<openssl::pkey::Private>, csr: &X509Req) {
    let mut pk_bytes = pkey.public_key_to_pem().unwrap();
    let mut csr_pem_bytes = csr.to_pem().unwrap();
    pk_bytes.append(&mut csr_pem_bytes);
    if let Ok(quote) = sgx_attest::sgx_attest(pk_bytes) {
        // handle quote
        log::info!("Writing quote to file: {}", quote_path.display());
        fs::write(quote_path, quote).await.expect("Failed to write quote to file");
    }
}
