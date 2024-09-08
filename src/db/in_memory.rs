use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{PendingNFT, TeleportDB, User, NFT};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct InMemoryDB {
    pub x_id_to_address: BTreeMap<String, String>,
    pub users: BTreeMap<String, User>,
    pub pending_nfts: BTreeMap<String, PendingNFT>,
    pub nfts: BTreeMap<String, NFT>,
    pub tweets: BTreeMap<String, String>,
    pub sessions: BTreeMap<String, String>,
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
    async fn add_user(&mut self, address: String, user: User) -> eyre::Result<()> {
        self.users.insert(address.clone(), user.clone());
        if let Some(x_id) = user.x_id {
            self.x_id_to_address.insert(x_id, address);
        }

        Ok(())
    }

    async fn get_user_by_address(&self, address: String) -> eyre::Result<User> {
        let user = self.users.get(&address).ok_or_else(|| eyre::eyre!("User not found"))?;
        Ok(user.clone())
    }

    async fn get_user_by_x_id(&self, x_id: String) -> eyre::Result<User> {
        let address = self
            .x_id_to_address
            .get(&x_id)
            .ok_or_else(|| eyre::eyre!("User address not found for x_id"))?;
        let user = self.users.get(address).ok_or_else(|| eyre::eyre!("User not found"))?;
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
        let nft = NFT { address: pending_nft.address, token_id };
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

    async fn add_session(&mut self, x_id: String) -> eyre::Result<String> {
        let session_id: i128 = rand::random();
        self.sessions.insert(session_id.to_string(), x_id);
        Ok(session_id.to_string())
    }

    async fn get_session(&self, session_id: String) -> eyre::Result<String> {
        let x_id = self.sessions.get(&session_id).ok_or_else(|| eyre::eyre!("Session not found"))?;
        Ok(x_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::AccessTokens;

    use super::*;

    #[tokio::test]
    async fn db_test_write() -> eyre::Result<()> {
        let mut db = InMemoryDB::new();
        let access_tokens = AccessTokens { token: "access token".to_string(), secret: "access secret".to_string() };
        let user = User {
            x_id: None,
            access_tokens: Some(access_tokens.clone()),
            oauth_tokens: access_tokens.clone(),
        };
        db.add_user("2".to_string(), user.clone()).await.expect("Failed to add user tokens");
        let user = db.get_user_by_address("2".to_string()).await?;
        assert_eq!(user.access_tokens.unwrap(), access_tokens);
        Ok(())
    }

    #[tokio::test]
    async fn db_test_overwrite() -> eyre::Result<()> {
        let mut db = InMemoryDB::new();
        let access_tokens = AccessTokens { token: "access token".to_string(), secret: "access secret".to_string() };
        let mut user = User {
            x_id: None,
            access_tokens: Some(access_tokens.clone()),
            oauth_tokens: access_tokens.clone(),
        };
        db.add_user("2".to_string(), user.clone()).await.expect("Failed to add user tokens");
        user.x_id = Some("1".to_string());
        db.add_user("2".to_string(), user.clone()).await.expect("Failed to add user tokens");
        let fetched_user = db.get_user_by_x_id("1".to_string()).await?;
        assert_eq!(user, fetched_user);
        Ok(())
    }
}
