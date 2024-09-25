use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{PendingNFT, Session, TeleportDB, User, NFT};

use rustls::ClientConfig;
use tokio_postgres_rustls::MakeRustlsConnect;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct InMemoryDB {
    pub x_id_to_address: BTreeMap<String, String>,
    pub users: BTreeMap<String, User>,
    pub pending_nfts: BTreeMap<String, PendingNFT>,
    pub nfts: BTreeMap<String, NFT>,
    pub tweets: BTreeMap<String, String>,
    pub sessions: BTreeMap<String, Session>,
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
        let nft = NFT { address: pending_nft.address, token_id: token_id.clone() };
        let nft_id_clone = pending_nft.nft_id.clone();
        self.nfts.insert(pending_nft.nft_id, nft);

        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let mut config = ClientConfig::new();
        config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let tls = MakeRustlsConnect::new(config);
        let (client, connection) = tokio_postgres::connect(&database_url, tls).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("connection error: {}", e);
            }
        });

        let token_id_int: i32 = token_id.parse().unwrap();
        // let nft_id_clone = pending_nft.nft_id;

        client
            .execute(
                "UPDATE \"NftIndex\" SET \"tokenId\" = $1 WHERE \"id\" = $2",
                &[&token_id_int, &nft_id_clone],
            )
            .await?;

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

    async fn add_session(&mut self, session: Session) -> eyre::Result<String> {
        let session_id: i128 = rand::random();
        self.sessions.insert(session_id.to_string(), session);
        Ok(session_id.to_string())
    }

    async fn get_session(&self, session_id: String) -> eyre::Result<Session> {
        let x_id =
            self.sessions.get(&session_id).ok_or_else(|| eyre::eyre!("Session not found"))?;
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
        let access_tokens =
            AccessTokens { token: "access token".to_string(), secret: "access secret".to_string() };
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
        let access_tokens =
            AccessTokens { token: "access token".to_string(), secret: "access secret".to_string() };
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
