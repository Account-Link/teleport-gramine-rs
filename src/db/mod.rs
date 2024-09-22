use rusqlite_from_row::FromRow;
use serde::{Deserialize, Serialize};
pub mod in_memory;
// pub mod sqlite;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct AccessTokens {
    pub token: String,
    pub secret: String,
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
}

pub trait TeleportDB {
    // async fn init(&mut self) -> eyre::Result<()>;
    // async fn open_from_file(file_path: &str) -> eyre::Result<Self>;
    async fn add_oauth(&mut self, token: String, secret: String) -> eyre::Result<()>;
    async fn get_oauth(&mut self, token: String) -> eyre::Result<String>;
    async fn add_user(&mut self, user: User) -> eyre::Result<()>;
    async fn get_user_by_x_id(&self, x_id: String) -> eyre::Result<User>;
    async fn add_pending_nft(
        &mut self,
        tx_hash: String,
        pending_nft: PendingNFT,
    ) -> eyre::Result<()>;
    async fn promote_pending_nft(&mut self, tx_hash: String, token_id: String) -> eyre::Result<()>;
    async fn get_nft(&self, nft_id: String) -> eyre::Result<NFT>;
    async fn add_tweet(&mut self, token_id: String, tweet_id: String) -> eyre::Result<()>;
    async fn get_tweet(&self, token_id: String) -> eyre::Result<String>;
    async fn add_session(&mut self, session: Session) -> eyre::Result<String>;
    async fn get_session(&self, session_id: String) -> eyre::Result<Session>;
    async fn serialize(&self) -> eyre::Result<Vec<u8>>;
}
