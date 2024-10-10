use std::sync::Arc;

use alloy::{
    providers::ProviderBuilder,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use config::{AppEnvironment, CONFIG};
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

    let twitter_builder = TwitterBuilder::new(
        &CONFIG.secrets.twitter_consumer_key,
        &CONFIG.secrets.twitter_consumer_secret,
    );

    let ws_rpc_url = format!("{}{}", CONFIG.app.ws_rpc_url, CONFIG.secrets.rpc_key);
    let rpc_url = format!("{}{}", CONFIG.app.rpc_url, CONFIG.secrets.rpc_key);

    let signer = MnemonicBuilder::<English>::default()
        .phrase(CONFIG.secrets.nft_minter_mnemonic.clone())
        .index(0)
        .unwrap()
        .build()
        .unwrap();

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(signer.clone().into())
        .on_http(rpc_url.parse().unwrap());

    let db = db::utils::load_or_create_db(&CONFIG.app.db_path).await;
    let db = Arc::new(Mutex::new(db));
    let openai_client = Arc::new(oai::OpenAIClient::new(&CONFIG.secrets.openai_api_key));
    let shared_state = Arc::new(SharedState {
        db: db.clone(),
        provider,
        signer,
        twitter_builder: twitter_builder.clone(),
        openai_client: openai_client.clone(),
        config: CONFIG.clone(),
    });

    let app = router::create_router(shared_state);

    match CONFIG.env {
        AppEnvironment::Production | AppEnvironment::Staging => {
            server_setup::setup_production_server(
                app,
                &CONFIG.tee.private_key_path,
                &CONFIG.tee.csr_save_path,
                &CONFIG.tee.quote_path,
                &CONFIG.app.backend_url,
                &CONFIG.tee.certificate_path,
            )
            .await?
        }
        AppEnvironment::Development => server_setup::setup_development_server(app).await?,
    }
    // spawn nft event subscription
    let db_clone = db.clone();
    let openai_client_clone = openai_client.clone();
    tokio::spawn(async move {
        subscribe_to_nft_events(
            db_clone,
            twitter_builder,
            ws_rpc_url,
            CONFIG.secrets.database_url.clone(),
            openai_client_clone,
            CONFIG.app.nft_address.clone(),
        )
        .await
        .unwrap();
    });

    // handle shutdown
    tokio::signal::ctrl_c().await.expect("failed to listen for event");
    db::utils::save_db_on_shutdown(db, &CONFIG.app.db_path).await;
    log::info!("Shutting down gracefully");

    Ok(())
}
