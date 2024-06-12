use std::path::Path;

use rusqlite::Connection;

pub fn create_tables(connection: &mut Connection) -> rusqlite::Result<()> {
    connection.execute(
        r#"
            CREATE TABLE IF NOT EXISTS twitter_tokens (
                id               INTEGER PRIMARY KEY,
                access_token     TEXT NOT NULL,
                access_secret    TEXT NOT NULL
            );
            "#,
        (),
    )?;
    log::info!("Initialized database tables");
    Ok(())
}

pub async fn add_user_tokens(
    connection: &mut Connection,
    user_id: u64,
    access_token: String,
    access_secret: String,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
            INSERT INTO twitter_tokens (id, access_token, access_secret)
            VALUES (?1, ?2, ?3)
            "#,
        rusqlite::params![user_id, access_token, access_secret],
    )?;
    log::info!("Added user tokens to database for user_id: {}", user_id);
    Ok(())
}

pub async fn get_user_tokens<P: AsRef<Path>>(
    db_url: P,
    user_id: u64,
) -> rusqlite::Result<(String, String)> {
    let connection = Connection::open(db_url)?;
    let mut stmt = connection.prepare(
        r#"
            SELECT access_token, access_secret
            FROM twitter_tokens
            WHERE id = ?1
            "#,
    )?;
    let mut rows = stmt.query(rusqlite::params![user_id])?;
    let row = rows
        .next()
        .expect("Failed to get row")
        .expect("No rows returned");
    let access_token: String = row.get(0)?;
    let access_secret: String = row.get(1)?;
    log::info!(
        "Retrieved user tokens from database for user_id: {}",
        user_id
    );
    Ok((access_token, access_secret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn db_test() -> eyre::Result<()> {
        env_logger::init();
        dotenv::dotenv().ok();
        let db_url = tempfile::NamedTempFile::new()?;
        let mut connection = Connection::open(db_url.path()).expect("Failed to open database");
        create_tables(&mut connection).expect("Failed to create tables");
        add_user_tokens(
            &mut connection,
            1,
            "access_token".to_string(),
            "access_secret".to_string(),
        )
        .await
        .expect("Failed to add user tokens");
        let (access_token, access_secret) = get_user_tokens(db_url.path(), 1)
            .await
            .expect("Failed to get user tokens");
        assert_eq!(access_token, "access_token");
        assert_eq!(access_secret, "access_secret");
        Ok(())
    }
}
