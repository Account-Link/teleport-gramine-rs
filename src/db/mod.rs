use rusqlite_from_row::FromRow;
use serde::{Deserialize, Serialize};
pub mod in_memory;
pub mod sqlite;

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, PartialEq, Eq)]
pub struct User {
    pub x_id: Option<String>,
    pub access_token: String,
    pub access_secret: String,
    pub address: String,
    pub sk: String,
}

pub trait UserDB {
    // async fn init(&mut self) -> eyre::Result<()>;
    // async fn open_from_file(file_path: &str) -> eyre::Result<Self>;
    async fn add_user(&mut self, teleport_id: String, user: User) -> eyre::Result<()>;
    async fn get_user_by_teleport_id(&self, teleport_id: String) -> eyre::Result<User>;
    async fn get_user_by_x_id(&self, x_id: String) -> eyre::Result<User>;
    async fn serialize(&self) -> eyre::Result<Vec<u8>>;
}
