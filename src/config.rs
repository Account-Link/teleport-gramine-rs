use std::path::PathBuf;

use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub env: AppEnvironment,
    pub tee: TEEConfig,
    pub app: AppConfig,
    #[serde(flatten)]
    pub secrets: Secrets,
}

#[derive(Debug, Deserialize)]
pub struct TEEConfig {
    pub private_key_path: PathBuf,
    pub certificate_path: PathBuf,
    pub csr_save_path: PathBuf,
    pub quote_path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub ws_rpc_url: String,
    pub rpc_url: String,
    pub backend_url: String,
    pub frontend_url: String,
    pub db_path: String,
    pub nft_address: String,
}

#[derive(Debug, Deserialize)]
pub struct Secrets {
    pub database_url: String,
    pub rpc_key: String,
    pub nft_minter_mnemonic: String,
    pub openai_api_key: String,
    pub twitter_consumer_key: String,
    pub twitter_consumer_secret: String,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppEnvironment {
    Development,
    Staging,
    Production,
}

impl Config {
    pub fn new() -> Result<Self, figment::Error> {
        dotenv::from_filename("private.env").ok();
        dotenv::dotenv().ok();

        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "default".to_string()).to_lowercase();

        let config = Figment::new()
            .merge(Toml::file("config.toml").nested())
            // merge the secrets from the environment variables
            .merge(Env::prefixed("APP_"))
            .select(env);

        log::info!("Loaded configuration: {:#?}", config);
        config.extract()
    }
}
