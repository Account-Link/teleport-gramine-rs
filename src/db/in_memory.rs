use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{User, UserDB};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct InMemoryUserDB {
    pub x_id_to_teleport_id: BTreeMap<String, String>,
    pub users: BTreeMap<String, User>,
}

impl InMemoryUserDB {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Failed to serialize InMemoryUserDB")
    }

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Failed to deserialize InMemoryUserDB")
    }
}

impl UserDB for InMemoryUserDB {
    async fn add_user(&mut self, teleport_id: String, user: User) -> eyre::Result<()> {
        self.users.insert(teleport_id.clone(), user.clone());
        if let Some(x_id) = user.x_id {
            self.x_id_to_teleport_id.insert(x_id, teleport_id);
        }

        Ok(())
    }

    async fn get_user_by_teleport_id(&self, teleport_id: String) -> eyre::Result<User> {
        let user = self
            .users
            .get(&teleport_id)
            .ok_or_else(|| eyre::eyre!("User not found"))?;
        Ok(user.clone())
    }

    async fn get_user_by_x_id(&self, x_id: String) -> eyre::Result<User> {
        let teleport_id = self
            .x_id_to_teleport_id
            .get(&x_id)
            .ok_or_else(|| eyre::eyre!("User teleport_id not found for x_id"))?;
        let user = self
            .users
            .get(teleport_id)
            .ok_or_else(|| eyre::eyre!("User not found"))?;
        Ok(user.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn db_test_write() -> eyre::Result<()> {
        let mut db = InMemoryUserDB::new();
        let user = User {
            x_id: None,
            access_token: "access token".to_string(),
            access_secret: "access secret".to_string(),
            address: "address".to_string(),
        };
        db.add_user("2".to_string(), user.clone())
            .await
            .expect("Failed to add user tokens");
        let user = db.get_user_by_teleport_id("2".to_string()).await?;
        assert_eq!(user.access_token, "access token");
        assert_eq!(user.access_secret, "access secret");
        assert_eq!(user.address, "address");
        Ok(())
    }

    #[tokio::test]
    async fn db_test_overwrite() -> eyre::Result<()> {
        let mut db = InMemoryUserDB::new();
        let mut user = User {
            x_id: None,
            access_token: "access token".to_string(),
            access_secret: "access secret".to_string(),
            address: "address".to_string(),
        };
        db.add_user("2".to_string(), user.clone())
            .await
            .expect("Failed to add user tokens");
        user.x_id = Some("1".to_string());
        db.add_user("2".to_string(), user.clone())
            .await
            .expect("Failed to add user tokens");
        let fetched_user = db.get_user_by_x_id("1".to_string()).await?;
        assert_eq!(user, fetched_user);
        Ok(())
    }
}
