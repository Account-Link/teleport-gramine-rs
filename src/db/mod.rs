use std::str::FromStr;

use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use rusqlite_from_row::FromRow;
use serde::{Deserialize, Serialize};
pub mod in_memory;
// pub mod sqlite;

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, PartialEq, Eq)]
pub struct User {
    pub x_id: Option<String>,
    pub access_token: String,
    pub access_secret: String,
    pub embedded_address: String,
    pub sk: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, PartialEq, Eq)]
pub struct NFT {
    pub teleport_id: String,
    pub token_id: String,
}

// #[derive(Debug, Serialize, Deserialize, Clone, FromRow, PartialEq, Eq)]
// pub struct PendingNFT {
//     pub teleport_id: String,
//     pub nft_id: String,
// }

impl User {
    pub fn signer(&self) -> eyre::Result<PrivateKeySigner> {
        let sk = self.sk.clone();
        if let Some(sk) = sk {
            Ok(PrivateKeySigner::from_str(&sk)?)
        } else {
            eyre::bail!("User does not have a private key")
        }
    }

    pub fn address(&self) -> eyre::Result<Address> {
        let signer = self.signer()?;
        Ok(signer.address())
    }
}

pub trait TeleportDB {
    // async fn init(&mut self) -> eyre::Result<()>;
    // async fn open_from_file(file_path: &str) -> eyre::Result<Self>;
    async fn add_user(&mut self, teleport_id: String, user: User) -> eyre::Result<()>;
    async fn get_user_by_teleport_id(&self, teleport_id: String) -> eyre::Result<User>;
    async fn get_user_by_x_id(&self, x_id: String) -> eyre::Result<User>;
    // async fn add_pending_nft(
    //     &mut self,
    //     tx_hash: String,
    //     pending_nft: PendingNFT,
    // ) -> eyre::Result<()>;
    // async fn promote_pending_nft(&mut self, tx_hash: String, token_id: String) -> eyre::Result<()>;
    async fn add_nft(&mut self, nft_id: String, nft: NFT) -> eyre::Result<()>;
    async fn get_nft(&self, nft_id: String) -> eyre::Result<NFT>;
    async fn add_tweet(&mut self, token_id: String, tweet_id: String) -> eyre::Result<()>;
    async fn get_tweet(&self, token_id: String) -> eyre::Result<String>;
    async fn serialize(&self) -> eyre::Result<Vec<u8>>;
}
