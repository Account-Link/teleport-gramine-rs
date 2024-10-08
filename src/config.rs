use std::path::PathBuf;

use config::File;
use serde::Deserialize;
use strum_macros::{Display, EnumString};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub tee: TEEConfig,
    pub env: Environment,

    // TODO: add proper nested config structure for env vars
    pub ws_rpc_url: String,
    pub rpc_url: String,
    pub tee_url: String,
    pub rpc_key: String,
    pub nft_minter_mnemonic: String,
    pub db_path: String,
    pub app_url: String,
    pub database_url: String,
    pub twitter_consumer_key: String,
    pub twitter_consumer_secret: String,
}

#[derive(Debug, Deserialize)]
pub struct TEEConfig {
    pub private_key_path: PathBuf,
    pub certificate_path: PathBuf,
    pub csr_save_path: PathBuf,
    pub quote_path: PathBuf,
}

#[derive(Debug, PartialEq, Deserialize, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Config {
    pub fn new() -> Result<Self, config::ConfigError> {
        dotenv::from_filename("private.env").ok();
        dotenv::dotenv().ok();

        let env = std::env::var("APP_ENV")
            .unwrap_or_else(|_| "development".into())
            .parse::<Environment>()
            .unwrap_or(Environment::Development);

        let builder = config::Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{env}")).required(false))
            .add_source(config::Environment::with_prefix("APP"));

        let config = builder.build()?;

        log::info!("Loaded configuration: {:#?}", config);

        config.try_deserialize()
    }
}
