use rusqlite::Connection;

pub fn open_connection(db_url: String) -> rusqlite::Result<Connection> {
    if db_url == "memory" {
        let connection = Connection::open_in_memory()?;
        log::info!("Opened in-memory database connection");
        return Ok(connection);
    }
    Connection::open(db_url)
}

pub fn create_tables(connection: &mut Connection) -> rusqlite::Result<()> {
    connection.execute(
        r#"
            CREATE TABLE IF NOT EXISTS twitter_access_tokens (
                x_id             INTEGER PRIMARY KEY,
                teleport_id      TEXT NOT NULL UNIQUE,
                access_token     TEXT NOT NULL,
                access_secret    TEXT NOT NULL
            );
            "#,
        (),
    )?;
    connection.execute(
        r#"
            CREATE TABLE IF NOT EXISTS twitter_oauth_tokens (
                id                    INTEGER PRIMARY KEY AUTOINCREMENT,
                teleport_id           TEXT NOT NULL UNIQUE,
                oauth_token           TEXT NOT NULL,
                oauth_token_secret    TEXT NOT NULL
            );
            "#,
        (),
    )?;
    log::info!("Initialized database tables");
    Ok(())
}

pub async fn add_access_tokens(
    connection: &mut Connection,
    x_id: String,
    teleport_id: String,
    access_token: String,
    access_secret: String,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
            REPLACE INTO twitter_access_tokens (x_id, teleport_id, access_token, access_secret)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        rusqlite::params![x_id, teleport_id, access_token, access_secret],
    )?;
    log::info!(
        "Added user tokens to database for teleport_id: {}",
        teleport_id
    );
    Ok(())
}

pub async fn add_oauth_tokens(
    connection: &mut Connection,
    teleport_id: String,
    oauth_token: String,
    oauth_token_secret: String,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
            REPLACE INTO twitter_oauth_tokens (teleport_id, oauth_token, oauth_token_secret)
            VALUES (?1, ?2, ?3)
            "#,
        rusqlite::params![teleport_id, oauth_token, oauth_token_secret],
    )?;
    log::info!(
        "Added oauth tokens in database for teleport_id: {}",
        teleport_id
    );
    Ok(())
}

pub async fn get_oauth_tokens_by_teleport_id(
    connection: &mut Connection,
    teleport_id: String,
) -> rusqlite::Result<(String, String)> {
    let mut stmt = connection.prepare(
        r#"
            SELECT oauth_token, oauth_token_secret
            FROM twitter_oauth_tokens
            WHERE teleport_id = ?1
            "#,
    )?;
    let mut rows = stmt.query(rusqlite::params![teleport_id])?;
    let row = rows
        .next()
        .expect("Failed to get row")
        .expect("No rows returned");
    let oauth_token: String = row.get(0)?;
    let oauth_token_secret: String = row.get(1)?;
    log::info!(
        "Retrieved oauth tokens from database for teleport_id: {}",
        teleport_id
    );
    Ok((oauth_token, oauth_token_secret))
}

pub async fn get_access_tokens(
    connection: &mut Connection,
    user_id: u64,
) -> rusqlite::Result<(String, String)> {
    let mut stmt = connection.prepare(
        r#"
            SELECT access_token, access_secret
            FROM twitter_access_tokens
            WHERE x_id = ?1
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
        let mut connection = Connection::open_in_memory()?;
        create_tables(&mut connection).expect("Failed to create tables");
        add_access_tokens(
            &mut connection,
            "1".to_string(),
            "2".to_string(),
            "access_token".to_string(),
            "access_secret".to_string(),
        )
        .await
        .expect("Failed to add user tokens");
        let (access_token, access_secret) = get_access_tokens(&mut connection, 1)
            .await
            .expect("Failed to get user tokens");
        assert_eq!(access_token, "access_token");
        assert_eq!(access_secret, "access_secret");
        Ok(())
    }
}
