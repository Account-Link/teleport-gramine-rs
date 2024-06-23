use reqwest_oauth1::OAuthClientProvider;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize)]
struct Tweet {
    text: String,
}

#[derive(Debug, Serialize)]
struct LikeTweet {
    tweet_id: String,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    data: UserInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub username: String,
    pub profile_image_url: String,
    // pub most_recent_tweet_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct RequestTokenRequestQuery {
    oauth_callback: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct RequestTokenResponseBody {
    oauth_token: String,
    oauth_token_secret: String,
    oauth_callback_confirmed: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct AccessTokenRequestQuery {
    oauth_verifier: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct AccessTokenResponseBody {
    oauth_token: String,
    oauth_token_secret: String,
    user_id: u64,
    screen_name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct CallbackUrlQuery {
    oauth_token: String,
    oauth_verifier: String,
}

pub async fn get_user_x_info(access_token: String, access_secret: String) -> UserInfo {
    let app_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let app_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");
    let client = reqwest::Client::new();
    let secrets =
        reqwest_oauth1::Secrets::new(app_key, app_secret).token(access_token, access_secret);
    let resp = client
        .oauth1(secrets)
        .get(
            "https://api.twitter.com/2/users/me?user.fields=profile_image_url,most_recent_tweet_id"
                .to_string(),
        )
        .send()
        .await
        .expect("Failed to get user info");
    let user_info: UserInfoResponse = resp.json().await.expect("Failed to parse user info");
    let user_info = user_info.data;
    log::info!("Fetched x_info: {:?}", user_info);
    user_info
}

pub async fn send_tweet(
    access_token: String,
    access_secret: String,
    tweet: String,
) -> eyre::Result<String> {
    let app_key = std::env::var("TWITTER_CONSUMER_KEY")?;
    let app_secret = std::env::var("TWITTER_CONSUMER_SECRET")?;
    let secrets =
        reqwest_oauth1::Secrets::new(app_key, app_secret).token(access_token, access_secret);
    let body = serde_json::to_string(&Tweet { text: tweet })?;
    let client = reqwest::Client::new();
    let resp = client
        .oauth1(secrets)
        .post("https://api.twitter.com/2/tweets".to_string())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await?;

    let tweet_response: SendTweetResponse = resp.json().await?;
    log::info!("Tweet response: {:?}", tweet_response);
    Ok(tweet_response.data.id)
}

pub async fn like_tweet(
    access_token: String,
    access_secret: String,
    x_id: String,
    tweet_id: String,
) -> eyre::Result<()> {
    let app_key = std::env::var("TWITTER_CONSUMER_KEY")?;
    let app_secret = std::env::var("TWITTER_CONSUMER_SECRET")?;
    let secrets =
        reqwest_oauth1::Secrets::new(app_key, app_secret).token(access_token, access_secret);
    let client = reqwest::Client::new();
    let resp = client
        .oauth1(secrets)
        .post(format!("https://api.twitter.com/2/users/{}/likes", x_id))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(serde_json::to_string(&&LikeTweet { tweet_id })?)
        .send()
        .await?;
    log::info!("Like response: {:?}", resp);
    Ok(())
}

pub async fn request_oauth_token(teleport_id: String) -> eyre::Result<(String, String)> {
    let app_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let app_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");
    let callback_url = format!("https://tee.teleport.best/callback?teleport_id={}", teleport_id);
    let secrets = reqwest_oauth1::Secrets::new(app_key, app_secret);
    let query = RequestTokenRequestQuery { oauth_callback: callback_url.to_string() };
    let response = reqwest_oauth1::Client::new()
        .post("https://api.twitter.com/oauth/request_token")
        .sign(secrets)
        .query(&query)
        .generate_signature()?
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        eyre::bail!(response.text().await?);
    }
    let response_bytes = response.bytes().await?;
    let request_token_body =
        serde_urlencoded::from_bytes::<RequestTokenResponseBody>(&response_bytes)?;
    assert!(request_token_body.oauth_callback_confirmed);
    Ok((request_token_body.oauth_token, request_token_body.oauth_token_secret))
}

pub async fn authorize_token(
    oauth_token: String,
    oauth_token_secret: String,
    oauth_verifier: String,
) -> eyre::Result<(String, String)> {
    let app_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let app_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");

    let query = AccessTokenRequestQuery { oauth_verifier };

    let secrets = reqwest_oauth1::Secrets::new(app_key, app_secret)
        .token(oauth_token.to_owned(), oauth_token_secret.to_owned());

    let response = reqwest_oauth1::Client::new()
        .post("https://api.twitter.com/oauth/access_token")
        .sign(secrets)
        .query(&query)
        .generate_signature()?
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        eyre::bail!(response.text().await?);
    }
    let response_bytes = response.bytes().await?;

    let access_token_body =
        serde_urlencoded::from_bytes::<AccessTokenResponseBody>(&response_bytes)?;

    Ok((access_token_body.oauth_token, access_token_body.oauth_token_secret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn e2e_oauth_test() {
        env_logger::init();
        dotenv::dotenv().ok();
        let tokens = request_oauth_token(1.to_string()).await.unwrap();
        log::info!("{:?}", tokens);
        let url =
            format!("https://api.twitter.com/oauth/authenticate?oauth_token={}", tokens.0.clone());
        log::info!("Please visit: {}", url);
        let mut callback_url = String::new();
        std::io::stdin().read_line(&mut callback_url).unwrap();
        let url = url::Url::parse(&callback_url).unwrap();
        let callback_url_query = url.query().unwrap_or_default();
        let callback_url_query: CallbackUrlQuery = serde_qs::from_str(callback_url_query).unwrap();
        let tokens =
            authorize_token(tokens.0, tokens.1, callback_url_query.oauth_verifier).await.unwrap();
        log::info!("{:?}", tokens);
    }

    #[tokio::test]
    #[ignore]
    async fn send_tweet_test() {
        env_logger::init();
        dotenv::dotenv().ok();
        let tweet_text = "Wow!".to_string();
        let access_token =
            std::env::var("TEST_ACCESS_TOKEN").expect("TEST_ACCESS_TOKEN not set").to_string();
        let access_secret =
            std::env::var("TEST_ACCESS_SECRET").expect("TEST_ACCESS_SECRET not set").to_string();
        let _ = send_tweet(access_token, access_secret, tweet_text).await;
    }

    #[tokio::test]
    // #[ignore]
    async fn like_tweet_test() {
        env_logger::init();
        dotenv::dotenv().ok();
        let access_token =
            std::env::var("TEST_ACCESS_TOKEN").expect("TEST_ACCESS_TOKEN not set").to_string();
        let access_secret =
            std::env::var("TEST_ACCESS_SECRET").expect("TEST_ACCESS_SECRET not set").to_string();
        let x_info = get_user_x_info(access_token.clone(), access_secret.clone()).await;
        let x_id = x_info.id;
        let _ =
            like_tweet(access_token, access_secret, x_id, "1803455775911694374".to_string()).await;
    }

    #[tokio::test]
    async fn get_user_info_test() {
        env_logger::init();
        dotenv::dotenv().ok();
        let access_token =
            std::env::var("TEST_ACCESS_TOKEN").expect("TEST_ACCESS_TOKEN not set").to_string();
        let access_secret =
            std::env::var("TEST_ACCESS_SECRET").expect("TEST_ACCESS_SECRET not set").to_string();
        get_user_x_info(access_token, access_secret).await;
    }
}
