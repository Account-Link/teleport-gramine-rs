use serde::Serialize;

use super::builder::TwitterClient;

#[derive(Debug, Serialize)]
struct LikeTweet {
    tweet_id: String,
}

impl TwitterClient<'_> {
    pub async fn like(&self, x_id: String, tweet_id: String) -> eyre::Result<()> {
        let _ = self
            .client
            .post(format!("https://api.twitter.com/2/users/{}/likes", x_id))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_string(&LikeTweet { tweet_id })?)
            .send()
            .await?;
        Ok(())
    }

    pub async fn retweet(&self, x_id: String, tweet_id: String) -> eyre::Result<()> {
        let _ = self
            .client
            .post(format!("https://api.twitter.com/2/users/{}/retweets", x_id))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_string(&LikeTweet { tweet_id })?)
            .send()
            .await?;
        Ok(())
    }
}
