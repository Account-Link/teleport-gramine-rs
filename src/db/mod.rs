use rusqlite_from_row::FromRow;
use serde::{Deserialize, Serialize};

use crate::twitter::auth::TwitterTokenPair;
pub mod in_memory;
// pub mod sqlite;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct AccessTokens {
    pub token: String,
    pub secret: String,
}

impl From<TwitterTokenPair> for AccessTokens {
    fn from(token_pair: TwitterTokenPair) -> Self {
        Self { token: token_pair.token, secret: token_pair.secret }
    }
}

impl From<AccessTokens> for TwitterTokenPair {
    fn from(access_tokens: AccessTokens) -> Self {
        Self { token: access_tokens.token, secret: access_tokens.secret }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct User {
    pub x_id: Option<String>,
    pub access_tokens: Option<AccessTokens>,
    pub oauth_tokens: AccessTokens,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, PartialEq, Eq)]
pub struct NFT {
    pub address: String,
    pub token_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, PartialEq, Eq)]
pub struct PendingNFT {
    pub address: String,
    pub nft_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, PartialEq, Eq)]
pub struct Session {
    pub x_id: String,
    pub address: String,
}

pub trait TeleportDB: Send + Sync + 'static {
    // async fn init(&mut self) -> eyre::Result<()>;
    // async fn open_from_file(file_path: &str) -> eyre::Result<Self>;
    fn add_user(&mut self, address: String, user: User) -> eyre::Result<()>;
    fn get_user_by_address(&self, address: String) -> eyre::Result<User>;
    fn get_user_by_x_id(&self, x_id: String) -> eyre::Result<User>;
    fn add_pending_nft(&mut self, tx_hash: String, pending_nft: PendingNFT) -> eyre::Result<()>;
    fn promote_pending_nft(&mut self, tx_hash: String, token_id: String) -> eyre::Result<String>;
    fn get_nft(&self, nft_id: String) -> eyre::Result<NFT>;
    fn add_tweet(&mut self, token_id: String, tweet_id: String) -> eyre::Result<()>;
    fn get_tweet(&self, token_id: String) -> eyre::Result<String>;
    fn add_session(&mut self, session: Session) -> eyre::Result<String>;
    fn get_session(&self, session_id: String) -> eyre::Result<Session>;
    fn serialize(&self) -> eyre::Result<Vec<u8>>;
}
