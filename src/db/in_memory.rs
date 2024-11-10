use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{read_to_string, File},
    io::Write,
    path::Path,
};

use super::{PendingNFT, Session, TeleportDB, User, NFT};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct InMemoryDB {
    pub x_id_to_address: BTreeMap<String, String>,
    pub oauths: BTreeMap<String, String>,
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
    fn add_oauth(&mut self, token: String, secret: String) -> eyre::Result<()> {
        self.oauths.insert(token, secret);
        Ok(())
    }

    fn get_oauth(&mut self, token: String) -> eyre::Result<String> {
        let secret = self.oauths.get(&token).ok_or_else(|| eyre::eyre!("OAuth not found"))?;
        Ok(secret.to_string())
    }

    fn add_user(&mut self, address: String, user: User) -> eyre::Result<()> {
        let file_path = Path::new("/root/shared/users").join(format!("{}.user", user.x_id.clone().unwrap()));
        log::info!("Saving user to file: {:?}", file_path.clone());
        let mut file = File::create(file_path)?;
        let contents = serde_json::to_string(&user)?;
        file.write_all(contents.as_bytes())?;
        // if let Some(x_id) = user.x_id {
        //     self.x_id_to_address.insert(x_id, address);
        // }
        //self.users.insert(user.x_id.clone().expect("no x_id"), user.clone());
        Ok(())
    }

    fn get_user_by_address(&self, address: String) -> eyre::Result<User> {
        let file_path = Path::new("/root/shared/users").join(format!("{}.user", address));
        let contents = read_to_string(file_path)?;
        let user: User = serde_json::from_str(&contents)?;
        Ok(user.clone())
    }

    fn get_user_by_x_id(&self, x_id: String) -> eyre::Result<User> {
        let file_path = Path::new("/root/shared/users").join(format!("{}.user", x_id));
        let contents = read_to_string(file_path)?;
        let user : User = serde_json::from_str(&contents)?;
        Ok(user.clone())
    }

    fn serialize(&self) -> eyre::Result<Vec<u8>> {
        let serialized = bincode::serialize(&self)?;
        Ok(serialized)
    }

    fn add_pending_nft(&mut self, tx_hash: String, pending_nft: PendingNFT) -> eyre::Result<()> {
        self.pending_nfts.insert(tx_hash, pending_nft);
        Ok(())
    }

    fn promote_pending_nft(&mut self, tx_hash: String, token_id: String) -> eyre::Result<String> {
        let pending_nft = self
            .pending_nfts
            .remove(&tx_hash)
            .ok_or_else(|| eyre::eyre!("Pending NFT not found"))?;
        let nft = NFT { address: pending_nft.address, token_id: token_id.clone() };
        let nft_id_clone = pending_nft.nft_id.clone();
        self.nfts.insert(pending_nft.nft_id, nft);

        Ok(nft_id_clone)
    }

    fn get_nft(&self, nft_id: String) -> eyre::Result<NFT> {
        let nft = self.nfts.get(&nft_id).ok_or_else(|| eyre::eyre!("NFT not found"))?;
        Ok(nft.clone())
    }

    fn add_tweet(&mut self, token_id: String, tweet_id: String) -> eyre::Result<()> {
        self.tweets.insert(token_id, tweet_id);
        Ok(())
    }

    fn get_tweet(&self, token_id: String) -> eyre::Result<String> {
        let tweet_id = self.tweets.get(&token_id).ok_or_else(|| eyre::eyre!("Tweet not found"))?;
        Ok(tweet_id.clone())
    }

    fn add_session(&mut self, session: Session) -> eyre::Result<String> {
        let session_id: i128 = rand::random();
        self.sessions.insert(session_id.to_string(), session);
        Ok(session_id.to_string())
    }

    fn get_session(&self, session_id: String) -> eyre::Result<Session> {
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
        db.add_user("2".to_string(), user.clone()).expect("Failed to add user tokens");
        let user = db.get_user_by_address("2".to_string())?;
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
        db.add_user("2".to_string(), user.clone()).expect("Failed to add user tokens");
        user.x_id = Some("1".to_string());
        db.add_user("2".to_string(), user.clone()).expect("Failed to add user tokens");
        let fetched_user = db.get_user_by_x_id("1".to_string())?;
        assert_eq!(user, fetched_user);
        Ok(())
    }
}
