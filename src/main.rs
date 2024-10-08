use std::{path::Path, sync::Arc};

use alloy::{
    providers::ProviderBuilder,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use endpoints::SharedState;
use tokio::sync::Mutex;
// Production-specific imports
#[cfg(feature = "production")]
use {
    crate::cert::create_csr, acme_lib::create_rsa_key, openssl::pkey::PKey, openssl::x509::X509Req,
};

use crate::{actions::nft::subscribe_to_nft_events, twitter::builder::TwitterBuilder};

// Common modules
mod actions;
mod config;
mod db;
mod endpoints;
mod oai;
mod server_setup;
mod templates;
pub mod twitter;

// Production-specific modules
#[cfg(feature = "production")]
mod cert;
#[cfg(feature = "production")]
mod sgx_attest;

mod router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = config::Config::new().expect("Failed to load configuration");

    let twitter_builder =
        TwitterBuilder::new(config.twitter_consumer_key, config.twitter_consumer_secret);

    let ws_rpc_url = format!("{}{}", config.ws_rpc_url, config.rpc_key);
    let rpc_url = format!("{}{}", config.rpc_url, config.rpc_key);

    #[cfg(feature = "production")]
    let private_key = load_or_create_private_key(&config.paths.private_key).await;
    #[cfg(feature = "production")]
    let csr = create_and_save_csr(&config.paths.csr, &config.tee_url, &private_key).await;
    #[cfg(feature = "production")]
    sgx_attest::handle_sgx_attestation(&config.paths.quote, &private_key, &csr).await;

    let signer = MnemonicBuilder::<English>::default()
        .phrase(config.nft_minter_mnemonic)
        .index(0)
        .unwrap()
        .build()
        .unwrap();

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(signer.clone().into())
        .on_http(rpc_url.parse().unwrap());

    let db = db::utils::load_or_create_db(&config.db_path).await;
    let db = Arc::new(Mutex::new(db));
    let shared_state = SharedState {
        db: db.clone(),
        provider,
        app_url: config.app_url,
        tee_url: config.tee_url,
        signer,
        twitter_builder: twitter_builder.clone(),
    };

    let app = router::create_router(shared_state);

    #[cfg(feature = "production")]
    server_setup::setup_server(app, private_key, config.paths.certificate).await?;

    #[cfg(not(feature = "production"))]
    server_setup::setup_server(app).await?;

    // spawn nft event subscription
    let db_clone = db.clone();
    tokio::spawn(async move {
        subscribe_to_nft_events(db_clone, twitter_builder, ws_rpc_url, config.database_url)
            .await
            .unwrap();
    });

    // handle shutdown
    tokio::signal::ctrl_c().await.expect("failed to listen for event");
    db::utils::save_db_on_shutdown(db, &config.db_path).await;
    log::info!("Shutting down gracefully");

    Ok(())
}

#[cfg(feature = "production")]
async fn load_or_create_private_key(private_key_path: &Path) -> PKey<openssl::pkey::Private> {
    if private_key_path.exists() {
        let pk_bytes = tokio::fs::read(private_key_path).await.expect("Failed to read pk file");
        PKey::private_key_from_pem(pk_bytes.as_slice()).unwrap()
    } else {
        let pk = create_rsa_key(2048);
        let pk_bytes = pk.private_key_to_pem_pkcs8().unwrap();
        tokio::fs::write(private_key_path, pk_bytes).await.expect("Failed to write pk to file");
        pk
    }
}

#[cfg(feature = "production")]
async fn create_and_save_csr(
    csr_path: &Path,
    tee_url: &str,
    private_key: &PKey<openssl::pkey::Private>,
) -> X509Req {
    let csr = create_csr(tee_url, private_key).unwrap();
    let csr_pem_bytes = csr.to_pem().unwrap();
    tokio::fs::write(csr_path, csr_pem_bytes).await.expect("Failed to write csr to file");
    csr
}
