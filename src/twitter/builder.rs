use oauth1_request::signature_method::hmac_sha1::HmacSha1;
use reqwest_oauth1::{Client, OAuthClientProvider, Secrets, Signer};

use super::auth::{self, TwitterTokenPair};

#[derive(Debug, Clone)]
pub struct TwitterBuilder {
    pub consumer_key: String,
    pub consumer_secret: String,
}

pub struct TwitterClient<'a> {
    pub client: Client<Signer<'a, Secrets<'a>, HmacSha1>>,
}

impl TwitterBuilder {
    pub fn new(consumer_key: &str, consumer_secret: &str) -> Self {
        Self {
            consumer_key: consumer_key.to_string(),
            consumer_secret: consumer_secret.to_string(),
        }
    }

    pub async fn request_oauth_token(
        &self,
        callback_url: String,
    ) -> eyre::Result<TwitterTokenPair> {
        auth::request_oauth_token(
            self.consumer_key.clone(),
            self.consumer_secret.clone(),
            callback_url,
        )
        .await
    }

    pub async fn authorize_token(
        &self,
        oauth_token: String,
        oauth_token_secret: String,
        oauth_verifier: String,
    ) -> eyre::Result<TwitterTokenPair> {
        auth::authorize_token(
            self.consumer_key.clone(),
            self.consumer_secret.clone(),
            oauth_token,
            oauth_token_secret,
            oauth_verifier,
        )
        .await
    }

    // pub fn from_access_tokens(tokens: AccessTokens) -> Self {

    // }

    pub fn with_auth(&self, tokens: TwitterTokenPair) -> TwitterClient {
        let secrets = Secrets::new(self.consumer_key.clone(), self.consumer_secret.clone())
            .token(tokens.token, tokens.secret);

        let client = reqwest::Client::new();
        // client.oauth1(secrets)
        TwitterClient { client: client.oauth1(secrets) }
    }
}
