use rustls::ClientConfig;
use tokio_postgres::Client;
use tokio_postgres_rustls::MakeRustlsConnect;

#[derive(Clone)]
pub struct ClientDB {
    database_url: String,
}

#[derive(Debug, Clone)]
pub struct TokenOwner {
    pub user_id: String,
    pub twitter_user_name: String,
}

impl ClientDB {
    pub fn new(database_url: String) -> Self {
        Self { database_url }
    }

    pub async fn client(&self) -> eyre::Result<Client> {
        let mut config = ClientConfig::new();
        config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let tls = MakeRustlsConnect::new(config);
        let (client, connection) = tokio_postgres::connect(&self.database_url, tls).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("connection error: {}", e);
            }
        });
        Ok(client)
    }

    pub async fn get_token_owner(&self, token_id: String) -> eyre::Result<TokenOwner> {
        let token_id_int: i32 = token_id.parse()?;
        let token_owner = self
            .client()
            .await?
            .query_one(
                "SELECT \"userId\", \"twitterUserName\" FROM \"NftIndex\" WHERE \"tokenId\" = $1",
                &[&token_id_int],
            )
            .await?;
        Ok(TokenOwner { user_id: token_owner.get(0), twitter_user_name: token_owner.get(1) })
    }

    pub async fn add_redeemed_tweet(
        &self,
        token_owner: TokenOwner,
        token_id: String,
        content: String,
        safeguard: String,
    ) -> eyre::Result<()> {
        let token_id_int: i32 = token_id.parse()?;
        let id = cuid::cuid2();

        self.client().await?.execute(
            "INSERT INTO \"RedeemedIndex\" (\"id\", \"creatorUserId\", \"tokenId\", \"tweetId\", \"twitterUserName\", \"safeguard\", \"content\") VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[&id, &token_owner.user_id, &token_id_int, &"".to_string(), &token_owner.twitter_user_name, &safeguard, &content],
        )
        .await?;
        Ok(())
    }

    pub async fn increment_user_redeemed(&self, user_id: String) -> eyre::Result<()> {
        self.client().await?
            .execute(
                "UPDATE \"User\" SET \"haveBeenRedeemed\" = \"haveBeenRedeemed\" + 1 WHERE \"id\" = $1",
                &[&user_id],
            )
            .await?;
        Ok(())
    }

    pub async fn set_token_id(&self, token_id: String, nft_id: String) -> eyre::Result<()> {
        let token_id_int: i32 = token_id.parse()?;
        self.client()
            .await?
            .execute(
                "UPDATE \"NftIndex\" SET \"tokenId\" = $1 WHERE \"id\" = $2",
                &[&token_id_int, &nft_id],
            )
            .await?;
        Ok(())
    }

    pub async fn delete_token(&self, token_id: String) -> eyre::Result<()> {
        let token_id_int: i32 = token_id.parse()?;
        self.client()
            .await?
            .execute("DELETE FROM \"NftIndex\" WHERE \"tokenId\" = $1", &[&token_id_int])
            .await?;
        Ok(())
    }

    pub async fn update_token_owner(&self, token_id: String, user_id: String) -> eyre::Result<()> {
        let token_id_int: i32 = token_id.parse()?;
        self.client()
            .await?
            .execute(
                "UPDATE \"NftIndex\" SET \"userId\" = $1 WHERE \"tokenId\" = $2",
                &[&user_id, &token_id_int],
            )
            .await?;
        Ok(())
    }
}
