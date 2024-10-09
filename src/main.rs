use std::{net::SocketAddr, path::Path, sync::Arc};

use acme_lib::create_rsa_key;
use alloy::signers::local::PrivateKeySigner;
use tokio::{sync::{mpsc,oneshot}, time::Duration};

use rand::Rng;
use axum::extract::{State};
use axum_server::tls_rustls::RustlsConfig;
use endpoints::{
    approve_mint, callback, cookietest, get_tweet_id, hello_world, mint, redeem, register_or_login,
    SharedState,
};
use openssl::pkey::{PKey,Private};
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

const PRIVATE_KEY_PATH: &str = "/root/save/private_key.pem";
const SHARED_KEY_PATH: &str = "/root/save/shared_key.pem";
const CERTIFICATE_PATH: &str = "untrustedhost/certificate.pem";
const CSR_PATH: &str = "untrustedhost/request.csr";
const QUOTE_PATH: &str = "untrustedhost/quote.dat";

const WALLET_PATH: &str = "/root/shared/wallet.key";

async fn generate_or_read_privkey() -> PKey<Private> {
    let tee_url = std::env::var("TEE_URL").expect("TEE_URL not set");
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
        log::info!("Writing quote to file: {}", QUOTE_PATH);
        fs::write(QUOTE_PATH, quote).await.expect("Failed to write quote to file");
    }
    pkey
}

async fn wait_for_cert() -> Vec<u8> {
    log::info!("Waiting for cert ...");
    while !Path::new(CERTIFICATE_PATH).exists() {
        sleep(Duration::from_secs(1)).await;
        }
    log::info!("Cert found");
    fs::read(CERTIFICATE_PATH).await.expect("cert not found")
}

async fn _handle_shared_key(State(s): State<Arc<Mutex<Option<oneshot::Sender<String>>>>>,
			    body: String)
			    -> String {
    if let Some(sender) = s.lock().await.take() {
        let _ = sender.send(body);
    }
    "ok".to_string()
}

async fn get_shared_key(cert: Vec<u8>, pkey: PKey<Private>) -> Vec<u8> {
    // Return the key if we already have it sealed
    if std::path::Path::new(SHARED_KEY_PATH).exists() {
	log::info!("reading from shared key file");
        let s = fs::read(SHARED_KEY_PATH).await.expect("couldn't read shared key");
        return s;
    }

    // Set up the oneshot channel
    let (tx, rx) = oneshot::channel();

    let shutdown_signal = Arc::new(Mutex::new(Some(tx)));

    // Define the route that handles the shared key
    let receive_app = axum::Router::new()
	.route("/shared_key", axum::routing::post(_handle_shared_key))
	.with_state(shutdown_signal);

    // Set up the Rustls config
    let config = RustlsConfig::from_pem(cert, pkey.private_key_to_pem_pkcs8().unwrap())
        .await
        .unwrap();

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8001));
    let server = axum_server::bind_rustls(addr, config)
        .serve(receive_app.into_make_service());

    // Spawn the server and stop it as soon as a key is received
    log::info!("waiting to receive shared key");
    let key_hex = tokio::select! {
        _ = server => todo!(),
        key_hex = rx => key_hex
    }.unwrap();
    
    // store the key
    log::info!("writing shared key");
    let key_bytes = hex::decode(key_hex).unwrap();
    fs::write(SHARED_KEY_PATH, &key_bytes)
        .await
        .expect("Failed to write shared key to file");
    key_bytes
}

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
    let do_bootstrap = std::env::var("BOOTSTRAP").is_ok();
    let do_onboard = std::env::var("ONBOARD").is_ok();
    let rpc_key = std::env::var("RPC_KEY").expect("RPC_KEY not set");
    let db_path = std::env::var("DB_PATH").expect("DB_PATH not set");
    let app_url = std::env::var("APP_URL").expect("APP_URL not set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let app_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let app_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");

    let twitter_builder = TwitterBuilder::new(app_key, app_secret);

    let ws_rpc_url = ws_rpc_url + &rpc_key;
    let rpc_url = rpc_url + &rpc_key;

    // TLS registration
    // Generate a private key and a fresh CSR
    let pkey = generate_or_read_privkey().await;

    // We need to wait for the certificate before we can receive the key
    let cert = wait_for_cert().await;

    // Get the shared key
    let shared_key = if do_bootstrap {
	// If we are bootstrapping, then generate shared secret
	let mut rng = rand::thread_rng();
	let mut shared_key = [0u8; 16];
	rng.fill(&mut shared_key);
	fs::write(SHARED_KEY_PATH, &shared_key)
            .await
            .expect("Failed to write shared key to file");
	shared_key.to_vec()
    } else {
	// Otherwise start a server and wait to receive it
	get_shared_key(cert, pkey.clone()).await
    };


    // Onboard others?
    if do_onboard {
	log::info!("sending the key to https://{}/shared_key/", tee_url);
	let url = format!("https://{}/shared_key", tee_url);
	let c = reqwest::Client::new().post(url).body(hex::encode(shared_key)).send().await.unwrap();
	log::info!("got: {}", c.text().await.unwrap());
	return;
    }

    // Unlock the encrypted files using the shared key
    log::info!("{} bytes", shared_key.len());
    fs::write("/dev/attestation/keys/shared", shared_key).await.expect("couldn't write to custom key");

    // Read or generate the signing key
    let signer = if std::path::Path::new(WALLET_PATH).exists() {
        let p_bytes = fs::read(WALLET_PATH).await.expect("failed to read wallet");
        PrivateKeySigner::from_slice(&p_bytes).unwrap()
    } else {
	// Generate a random wallet (24 word phrase)
        let signer = PrivateKeySigner::random();
        fs::write(WALLET_PATH, signer.to_bytes()).await.expect("failed to write wallet");
        signer
    };
    log::info!("Signer address:{}", signer.address());

    let provider = get_provider(rpc_url.clone(), signer.clone().into());

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
        twitter_builder: twitter_builder.clone(),
        nft_action_sender: sender,
	rpc_url: rpc_url,
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
    nft_action_consumer(receiver, provider).await
}
