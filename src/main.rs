use std::sync::Arc;

use alloy::{
    providers::ProviderBuilder,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use endpoints::SharedState;
use tokio::sync::Mutex;

use crate::{actions::nft::subscribe_to_nft_events, twitter::builder::TwitterBuilder};

// Common modules
mod actions;
mod config;
mod db;
mod endpoints;
mod oai;
mod router;
mod server_setup;
mod templates;
pub mod twitter;

// Production-specific modules
mod cert;
mod sgx_attest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = config::Config::new().expect("Failed to load configuration");

    let twitter_builder =
        TwitterBuilder::new(&config.twitter_consumer_key, &config.twitter_consumer_secret);

    let ws_rpc_url = format!("{}{}", config.ws_rpc_url, config.rpc_key);
    let rpc_url = format!("{}{}", config.rpc_url, config.rpc_key);

    let signer = MnemonicBuilder::<English>::default()
        .phrase(config.nft_minter_mnemonic.clone())
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
        app_url: config.app_url.clone(),
        tee_url: config.tee_url.clone(),
        signer,
        twitter_builder: twitter_builder.clone(),
    };

    let app = router::create_router(shared_state);

    match config.env {
        config::Environment::Production | config::Environment::Staging => {
            server_setup::setup_production_server(
                app,
                &config.tee.private_key_path,
                &config.tee.csr_save_path,
                &config.tee.quote_path,
                &config.tee_url,
                &config.tee.certificate_path,
            )
            .await?
        }
        config::Environment::Development => server_setup::setup_development_server(app).await?,
    }
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
