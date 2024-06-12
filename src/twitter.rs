use reqwest_oauth1::OAuthClientProvider;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize)]
struct Tweet {
    text: String,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    data: UserInfo,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    id: String,
    name: String,
    username: String,
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

//
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

pub async fn get_user_id(access_token: String, access_secret: String) -> u64 {
    let app_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let app_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");
    let client = reqwest::Client::new();
    let secrets =
        reqwest_oauth1::Secrets::new(app_key, app_secret).token(access_token, access_secret);
    let resp = client
        .oauth1(secrets)
        .get("https://api.twitter.com/2/users/me".to_string())
        .send()
        .await
        .expect("Failed to get user info");
    let user_info: UserInfoResponse = resp.json().await.expect("Failed to parse user info");
    let id: u64 = user_info.data.id.parse().expect("Failed to parse user id");
    log::info!("{:?}", user_info);
    log::info!("User id: {}", id);
    id
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

pub async fn request_oauth_token() -> eyre::Result<(String, String)> {
    let app_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let app_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");
    let callback_url = "http://127.0.0.1:5000/login/callback";
    let secrets = reqwest_oauth1::Secrets::new(app_key, app_secret);
    let query = RequestTokenRequestQuery {
        oauth_callback: callback_url.to_string(),
    };
    let response = reqwest_oauth1::Client::new()
        .post("https://api.twitter.com/oauth/request_token")
        .sign(secrets)
        .query(&query)
        .generate_signature()?
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(eyre::eyre!(response.text().await?));
    }
    let response_bytes = response.bytes().await?;
    let request_token_body =
        serde_urlencoded::from_bytes::<RequestTokenResponseBody>(&response_bytes)?;
    assert!(request_token_body.oauth_callback_confirmed);
    Ok((
        request_token_body.oauth_token,
        request_token_body.oauth_token_secret,
    ))
}

pub async fn authorize_token(
    oauth_token: String,
    oauth_token_secret: String,
    callback_url: Url,
) -> eyre::Result<(String, String)> {
    let app_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let app_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");
    let callback_url_query = callback_url.query().unwrap_or_default();
    let callback_url_query: CallbackUrlQuery = serde_qs::from_str(callback_url_query)?;
    assert_eq!(callback_url_query.oauth_token, oauth_token);

    let query = AccessTokenRequestQuery {
        oauth_verifier: callback_url_query.oauth_verifier.to_owned(),
    };

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
        return Err(eyre::eyre!(response.text().await?));
    }
    let response_bytes = response.bytes().await?;

    let access_token_body =
        serde_urlencoded::from_bytes::<AccessTokenResponseBody>(&response_bytes)?;

    Ok((
        access_token_body.oauth_token,
        access_token_body.oauth_token_secret,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn e2e_oauth_test() {
        env_logger::init();
        dotenv::dotenv().ok();
        let tokens = request_oauth_token().await.unwrap();
        log::info!("{:?}", tokens);
        let url = format!(
            "https://api.twitter.com/oauth/authenticate?oauth_token={}",
            tokens.0.clone()
        );
        log::info!("Please visit: {}", url);
        let mut callback_url = String::new();
        std::io::stdin().read_line(&mut callback_url).unwrap();
        let url = Url::parse(&callback_url).unwrap();
        let tokens = authorize_token(tokens.0, tokens.1, url).await.unwrap();
        log::info!("{:?}", tokens);
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

    #[tokio::test]
    async fn get_user_info_test() {
        env_logger::init();
        dotenv::dotenv().ok();
        let access_token = std::env::var("TEST_ACCESS_TOKEN")
            .expect("TEST_ACCESS_TOKEN not set")
            .to_string();
        let access_secret = std::env::var("TEST_ACCESS_SECRET")
            .expect("TEST_ACCESS_SECRET not set")
            .to_string();
        get_user_id(access_token, access_secret).await;
    }
}
