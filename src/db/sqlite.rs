use std::ops::Deref;

use rusqlite::{Connection, DatabaseName, OpenFlags};
use rusqlite_from_row::FromRow;

use super::{User, UserDB};

pub struct SqliteUserDB {
    pub connection: Connection,
}

impl SqliteUserDB {
    pub fn new(path: String) -> eyre::Result<Self> {
        let flags = if path == "memdb" {
            OpenFlags::SQLITE_OPEN_MEMORY
                | OpenFlags::SQLITE_OPEN_SHARED_CACHE
                | OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
        } else {
            OpenFlags::default()
        };
        let connection =
            Connection::open_with_flags(&path, flags).expect("Failed to open database");
        connection.execute(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id               INTEGER PRIMARY KEY AUTOINCREMENT,
                x_id             TEXT UNIQUE,
                teleport_id      TEXT NOT NULL UNIQUE,
                access_token     TEXT NOT NULL,
                access_secret    TEXT NOT NULL,
                embedded_address          TEXT NOT NULL,
                sk               TEXT
            );
            "#,
            (),
        )?;
        log::info!("Initialized database tables");
        Ok(Self { connection })
    }
}

impl UserDB for SqliteUserDB {
    async fn add_user(&mut self, teleport_id: String, user: User) -> eyre::Result<()> {
        self.connection.execute(
            r#"
            REPLACE INTO users (x_id, teleport_id, access_token, access_secret, embedded_address, sk)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            rusqlite::params![
                user.x_id,
                teleport_id,
                user.access_token,
                user.access_secret,
                user.embedded_address,
                user.sk
            ],
        )?;
        log::info!(
            "Added user tokens to database for teleport_id: {}",
            teleport_id
        );
        Ok(())
    }

    async fn get_user_by_teleport_id(&self, teleport_id: String) -> eyre::Result<User> {
        let mut stmt = self.connection.prepare(
            r#"
            SELECT x_id, teleport_id, access_token, access_secret, embedded_address, sk
            FROM users
            WHERE teleport_id = ?1
            "#,
        )?;
        let mut rows = stmt.query(rusqlite::params![teleport_id])?;
        let row = rows
            .next()
            .expect("Failed to get row")
            .expect("No rows returned");
        let user = User::try_from_row(row)?;
        log::info!(
            "Retrieved user tokens from database for teleport_id: {}",
            teleport_id
        );
        Ok(user)
    }

    async fn get_user_by_x_id(&self, x_id: String) -> eyre::Result<User> {
        let mut stmt = self.connection.prepare(
            r#"
            SELECT x_id, teleport_id, access_token, access_secret, embedded_address, sk
            FROM users
            WHERE x_id = ?1
            "#,
        )?;
        let mut rows = stmt.query(rusqlite::params![x_id])?;
        let row = rows
            .next()
            .expect("Failed to get row")
            .expect("No rows returned");
        let user = User::try_from_row(row)?;
        log::info!("Retrieved user tokens from database for x_id: {}", x_id);
        Ok(user)
    }

    async fn serialize(&self) -> eyre::Result<Vec<u8>> {
        let serialized = self.connection.serialize(DatabaseName::Main)?;
        let vec = serialized.deref().to_vec();
        Ok(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn db_test_write() -> eyre::Result<()> {
        let mut db = SqliteUserDB::new("memdb".to_string())?;
        let user = User {
            x_id: None,
            access_token: "access token".to_string(),
            access_secret: "access secret".to_string(),
            embedded_address: "address".to_string(),
            sk: None,
        };
        db.add_user("2".to_string(), user.clone())
            .await
            .expect("Failed to add user tokens");
        let user = db.get_user_by_teleport_id("2".to_string()).await?;
        assert_eq!(user.access_token, "access token");
        assert_eq!(user.access_secret, "access secret");
        assert_eq!(user.embedded_address, "address");
        Ok(())
    }

    #[tokio::test]
    async fn db_test_overwrite() -> eyre::Result<()> {
        let mut db = SqliteUserDB::new("memdb".to_string())?;
        let mut user = User {
            x_id: None,
            access_token: "access token".to_string(),
            access_secret: "access secret".to_string(),
            embedded_address: "address".to_string(),
            sk: None,
        };
        db.add_user("2".to_string(), user.clone())
            .await
            .expect("Failed to add user tokens");
        user.x_id = Some("1".to_string());
        user.sk = Some("sk".to_string());
        db.add_user("2".to_string(), user.clone())
            .await
            .expect("Failed to add user tokens");
        let fetched_user = db.get_user_by_x_id("1".to_string()).await?;
        assert_eq!(user, fetched_user);
        Ok(())
    }
}
