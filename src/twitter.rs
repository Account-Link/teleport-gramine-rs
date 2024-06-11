use reqwest_oauth1::OAuthClientProvider;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Tweet {
    pub text: String,
}

pub async fn send_tweet(access_token: String, access_secret: String, tweet: String) {
    let app_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let app_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");
    let secrets =
        reqwest_oauth1::Secrets::new(app_key, app_secret).token(access_token, access_secret);
    let body = serde_json::to_string(&Tweet { text: tweet }).unwrap();
    let client = reqwest::Client::new();
    let resp = client
        .oauth1(secrets)
        .post("https://api.twitter.com/2/tweets".to_string())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .expect("Failed to send tweet");
    log::info!("{:?}", resp.text().await);
}

#[tokio::test]
async fn send_tweet_test() {
    env_logger::init();
    dotenv::dotenv().ok();
    let tweet_text = "Wow!".to_string();
    let access_token = std::env::var("TEST_ACCESS_TOKEN")
        .expect("TEST_ACCESS_TOKEN not set")
        .to_string();
    let access_secret = std::env::var("TEST_ACCESS_SECRET")
        .expect("TEST_ACCESS_SECRET not set")
        .to_string();
    send_tweet(access_token, access_secret, tweet_text).await;
}
