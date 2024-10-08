use serde::{Deserialize, Serialize};

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
struct CallbackUrlQuery {
    oauth_token: String,
    oauth_verifier: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TwitterTokenPair {
    pub token: String,
    pub secret: String,
}

pub async fn request_oauth_token(
    app_key: String,
    app_secret: String,
    callback_url: String,
) -> eyre::Result<TwitterTokenPair> {
    let secrets = reqwest_oauth1::Secrets::new(app_key, app_secret);
    log::info!("Requesting OAuth token");
    log::info!("Callback URL: {}", callback_url);
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
    Ok(TwitterTokenPair {
        token: request_token_body.oauth_token,
        secret: request_token_body.oauth_token_secret,
    })
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct AccessTokenResponseBody {
    oauth_token: String,
    oauth_token_secret: String,
    user_id: u64,
    screen_name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct AccessTokenRequestQuery {
    oauth_verifier: String,
}

pub async fn authorize_token(
    app_key: String,
    app_secret: String,
    oauth_token: String,
    oauth_token_secret: String,
    oauth_verifier: String,
) -> eyre::Result<TwitterTokenPair> {
    let query = AccessTokenRequestQuery { oauth_verifier };

    let secrets =
        reqwest_oauth1::Secrets::new(app_key, app_secret).token(oauth_token, oauth_token_secret);

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

    Ok(TwitterTokenPair {
        token: access_token_body.oauth_token,
        secret: access_token_body.oauth_token_secret,
    })
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[tokio::test]
//     #[ignore]
//     async fn e2e_oauth_test() {
//         env_logger::init();
//         dotenv::dotenv().ok();
//         let tokens = request_oauth_token(1.to_string()).await.unwrap();
//         // log::info!("{:?}", tokens);
//         let url = format!(
//             "https://api.twitter.com/oauth/authenticate?oauth_token={}",
//             tokens.token.clone()
//         );
//         log::info!("Please visit: {}", url);
//         let mut callback_url = String::new();
//         std::io::stdin().read_line(&mut callback_url).unwrap();
//         let url = url::Url::parse(&callback_url).unwrap();
//         let callback_url_query = url.query().unwrap_or_default();
//         let callback_url_query: CallbackUrlQuery =
// serde_qs::from_str(callback_url_query).unwrap();         let tokens = authorize_token(
//             tokens.token,
//             tokens.secret,
//             callback_url_query.oauth_verifier,
//         )
//         .await
//         .unwrap();
//         // log::info!("{:?}", tokens);
//     }
// }
