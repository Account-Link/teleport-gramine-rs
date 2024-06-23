use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{PendingNFT, TeleportDB, User, NFT};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct InMemoryDB {
    pub x_id_to_teleport_id: BTreeMap<String, String>,
    pub users: BTreeMap<String, User>,
    pub pending_nfts: BTreeMap<String, PendingNFT>,
    pub nfts: BTreeMap<String, NFT>,
    pub tweets: BTreeMap<String, String>,
}

impl InMemoryDB {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Failed to deserialize InMemoryUserDB")
    }
}

impl TeleportDB for InMemoryDB {
    async fn add_user(&mut self, teleport_id: String, user: User) -> eyre::Result<()> {
        self.users.insert(teleport_id.clone(), user.clone());
        if let Some(x_id) = user.x_id {
            self.x_id_to_teleport_id.insert(x_id, teleport_id);
        }

        Ok(())
    }

    async fn get_user_by_teleport_id(&self, teleport_id: String) -> eyre::Result<User> {
        let user = self.users.get(&teleport_id).ok_or_else(|| eyre::eyre!("User not found"))?;
        Ok(user.clone())
    }

    async fn get_user_by_x_id(&self, x_id: String) -> eyre::Result<User> {
        let teleport_id = self
            .x_id_to_teleport_id
            .get(&x_id)
            .ok_or_else(|| eyre::eyre!("User teleport_id not found for x_id"))?;
        let user = self.users.get(teleport_id).ok_or_else(|| eyre::eyre!("User not found"))?;
        Ok(user.clone())
    }

    async fn serialize(&self) -> eyre::Result<Vec<u8>> {
        let serialized = bincode::serialize(&self)?;
        Ok(serialized)
    }

    async fn add_pending_nft(
        &mut self,
        tx_hash: String,
        pending_nft: PendingNFT,
    ) -> eyre::Result<()> {
        self.pending_nfts.insert(tx_hash, pending_nft);
        Ok(())
    }

    async fn promote_pending_nft(&mut self, tx_hash: String, token_id: String) -> eyre::Result<()> {
        let pending_nft = self
            .pending_nfts
            .remove(&tx_hash)
            .ok_or_else(|| eyre::eyre!("Pending NFT not found"))?;
        let nft = NFT { teleport_id: pending_nft.teleport_id, token_id };
        self.nfts.insert(pending_nft.nft_id, nft);
        Ok(())
    }

    async fn get_nft(&self, nft_id: String) -> eyre::Result<NFT> {
        let nft = self.nfts.get(&nft_id).ok_or_else(|| eyre::eyre!("NFT not found"))?;
        Ok(nft.clone())
    }

    async fn add_tweet(&mut self, token_id: String, tweet_id: String) -> eyre::Result<()> {
        self.tweets.insert(token_id, tweet_id);
        Ok(())
    }

    async fn get_tweet(&self, token_id: String) -> eyre::Result<String> {
        let tweet_id = self.tweets.get(&token_id).ok_or_else(|| eyre::eyre!("Tweet not found"))?;
        Ok(tweet_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn db_test_write() -> eyre::Result<()> {
        let mut db = InMemoryDB::new();
        let user = User {
            x_id: None,
            access_token: "access token".to_string(),
            access_secret: "access secret".to_string(),
            embedded_address: "address".to_string(),
            sk: None,
        };
        db.add_user("2".to_string(), user.clone()).await.expect("Failed to add user tokens");
        let user = db.get_user_by_teleport_id("2".to_string()).await?;
        assert_eq!(user.access_token, "access token");
        assert_eq!(user.access_secret, "access secret");
        assert_eq!(user.embedded_address, "address");
        Ok(())
    }

    #[tokio::test]
    async fn db_test_overwrite() -> eyre::Result<()> {
        let mut db = InMemoryDB::new();
        let mut user = User {
            x_id: None,
            access_token: "access token".to_string(),
            access_secret: "access secret".to_string(),
            embedded_address: "address".to_string(),
            sk: None,
        };
        db.add_user("2".to_string(), user.clone()).await.expect("Failed to add user tokens");
        user.x_id = Some("1".to_string());
        user.sk = Some("sk".to_string());
        db.add_user("2".to_string(), user.clone()).await.expect("Failed to add user tokens");
        let fetched_user = db.get_user_by_x_id("1".to_string()).await?;
        assert_eq!(user, fetched_user);
        Ok(())
    }
}
