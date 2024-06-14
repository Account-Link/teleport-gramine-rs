use rusqlite::Connection;
use rusqlite_from_row::FromRow;

#[derive(Debug, FromRow)]
pub struct User {
    pub x_id: String,
    pub teleport_id: String,
    pub access_token: String,
    pub access_secret: String,
    pub address: String,
}

#[derive(Debug, FromRow)]
pub struct OAuthUser {
    pub teleport_id: String,
    pub oauth_token: String,
    pub oauth_token_secret: String,
    pub address: String,
}

pub fn open_connection(db_url: String) -> rusqlite::Result<Connection> {
    if db_url == "memory" {
        let connection = Connection::open_in_memory()?;
        log::info!("Opened in-memory database connection");
        return Ok(connection);
    }
    Connection::open(db_url)
}

pub fn create_tables(connection: &mut Connection) -> eyre::Result<()> {
    connection.execute(
        r#"
            CREATE TABLE IF NOT EXISTS twitter_access_tokens (
                id               INTEGER PRIMARY KEY AUTOINCREMENT,
                x_id             TEXT NOT NULL UNIQUE,
                teleport_id      TEXT NOT NULL UNIQUE,
                access_token     TEXT NOT NULL,
                access_secret    TEXT NOT NULL,
                address          TEXT NOT NULL
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
                oauth_token_secret    TEXT NOT NULL,
                address               TEXT NOT NULL
            );
            "#,
        (),
    )?;
    log::info!("Initialized database tables");
    Ok(())
}

pub async fn add_user(connection: &mut Connection, user: User) -> eyre::Result<()> {
    connection.execute(
        r#"
            REPLACE INTO twitter_access_tokens (x_id, teleport_id, access_token, access_secret, address)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        rusqlite::params![
            user.x_id,
            user.teleport_id,
            user.access_token,
            user.access_secret,
            user.address
        ],
    )?;
    log::info!(
        "Added user tokens to database for teleport_id: {}",
        user.teleport_id
    );
    Ok(())
}

pub async fn add_oauth_user(
    connection: &mut Connection,
    oauth_user: OAuthUser,
) -> eyre::Result<()> {
    connection.execute(
        r#"
            REPLACE INTO twitter_oauth_tokens (teleport_id, oauth_token, oauth_token_secret, address)
            VALUES (?1, ?2, ?3)
            "#,
        rusqlite::params![oauth_user.teleport_id, oauth_user.oauth_token, oauth_user.oauth_token_secret, oauth_user.address],
    )?;
    log::info!(
        "Added oauth tokens in database for teleport_id: {}",
        oauth_user.teleport_id
    );
    Ok(())
}

pub async fn get_oauth_user_by_teleport_id(
    connection: &mut Connection,
    teleport_id: String,
) -> eyre::Result<OAuthUser> {
    let mut stmt = connection.prepare(
        r#"
            SELECT teleport_id, oauth_token, oauth_token_secret, address
            FROM twitter_oauth_tokens
            WHERE teleport_id = ?1
            "#,
    )?;
    let mut rows = stmt.query(rusqlite::params![teleport_id])?;
    let row = rows
        .next()
        .expect("Failed to get row")
        .expect("No rows returned");
    let user = OAuthUser::try_from_row(row)?;
    log::info!(
        "Retrieved oauth tokens from database for teleport_id: {}",
        teleport_id
    );
    Ok(user)
}

pub async fn get_user_by_teleport_id(
    connection: &mut Connection,
    teleport_id: String,
) -> eyre::Result<User> {
    let mut stmt = connection.prepare(
        r#"
            SELECT x_id, teleport_id, access_token, access_secret, address
            FROM twitter_access_tokens
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

pub async fn get_user_by_x_id(connection: &mut Connection, x_id: String) -> eyre::Result<User> {
    let mut stmt = connection.prepare(
        r#"
            SELECT x_id, teleport_id, access_token, access_secret, address
            FROM twitter_access_tokens
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn db_test() -> eyre::Result<()> {
        env_logger::init();
        dotenv::dotenv().ok();
        let mut connection = Connection::open_in_memory()?;
        create_tables(&mut connection).expect("Failed to create tables");
        let user = User {
            x_id: "1".to_string(),
            teleport_id: "2".to_string(),
            access_token: "access token".to_string(),
            access_secret: "access secret".to_string(),
            address: "address".to_string(),
        };
        add_user(&mut connection, user)
            .await
            .expect("Failed to add user tokens");
        let user = get_user_by_teleport_id(&mut connection, "2".to_string())
            .await
            .expect("Failed to get user tokens");
        assert_eq!(user.access_token, "access_token");
        assert_eq!(user.access_secret, "access_secret");
        assert_eq!(user.address, "address");
        Ok(())
    }
}
