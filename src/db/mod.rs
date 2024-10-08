pub mod client_db;
pub mod in_memory;
pub mod models;
pub mod utils;
// pub mod sqlite;

pub use models::*;

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
