use serde::Deserialize;

use super::{builder::TwitterClient, tweet::Tweet};

#[derive(Debug, Deserialize)]
struct SendTweetData {
    // edit_history_tweet_ids: Vec<String>,
    // text: String,
    id: String,
}

#[derive(Debug, Deserialize)]
struct SendTweetResponse {
    data: SendTweetData,
}

#[derive(Deserialize, Debug)]
struct MediaUploadResponse {
    // media_data: String,
    media_id_string: String,
}

impl TwitterClient<'_> {
    pub async fn raw_tweet(&self, tweet: Tweet) -> eyre::Result<String> {
        tweet.validate()?;
        let body = serde_json::to_string(&tweet)?;
        let resp = self
            .client
            .post("https://api.twitter.com/2/tweets".to_string())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await?;

        let tweet_response: SendTweetResponse = resp.json().await?;
        log::info!("Tweet response: {:?}", tweet_response);
        Ok(tweet_response.data.id)
    }

    pub async fn upload_media(&self, media_bytes: Vec<u8>) -> eyre::Result<String> {
        let form = reqwest::multipart::Form::new()
            .part("media", reqwest::multipart::Part::bytes(media_bytes));
        let resp = self
            .client
            .post("https://upload.twitter.com/1.1/media/upload.json".to_string())
            .multipart(form)
            .send()
            .await?;
        let media_upload_response: MediaUploadResponse = resp.json().await?;
        Ok(media_upload_response.media_id_string)
    }

    // pub async fn tweet(&self, tweet: String) -> eyre::Result<String> {
    //     let tweet = Tweet::new(tweet);
    //     self.raw_tweet(tweet).await
    // }

    // pub async fn tweet_with_media(
    //     &self,
    //     tweet: String,
    //     media_ids: Vec<String>,
    // ) -> eyre::Result<String> {
    //     let tweet = Tweet::new(tweet).use_media_ids(media_ids);
    //     self.raw_tweet(tweet).await
    // }

    // pub async fn quote(&self, tweet: String, quote_tweet_id: String) -> eyre::Result<String> {
    //     let tweet = Tweet::new(tweet).use_quote_tweet_id(quote_tweet_id);
    //     self.raw_tweet(tweet).await
    // }

    // pub async fn quote_tweet_with_media(
    //     &self,
    //     tweet: String,
    //     quote_tweet_id: String,
    //     media_ids: Vec<String>,
    // ) -> eyre::Result<String> {
    //     let tweet = Tweet::new(tweet)
    //         .use_quote_tweet_id(quote_tweet_id)
    //         .use_media_ids(media_ids);
    //     self.raw_tweet(tweet).await
    // }

    // pub async fn reply(&self, tweet: String, reply_tweet_id: String) -> eyre::Result<String> {
    //     let tweet = Tweet::new(tweet).use_reply_tweet_id(reply_tweet_id);
    //     self.raw_tweet(tweet).await
    // }

    // pub async fn reply_with_media(
    //     &self,
    //     tweet: String,
    //     reply_tweet_id: String,
    //     media_ids: Vec<String>,
    // ) -> eyre::Result<String> {
    //     let tweet = Tweet::new(tweet)
    //         .use_reply_tweet_id(reply_tweet_id)
    //         .use_media_ids(media_ids);
    //     self.raw_tweet(tweet).await
    // }
}
