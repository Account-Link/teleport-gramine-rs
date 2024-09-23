pub mod auth;
pub mod builder;
pub mod info;
pub mod post;
pub mod react;
pub mod tweet;

pub fn get_callback_url(
    callback_base_url: String,
    address: String,
    frontend_nonce: String,
) -> String {
    format!(
        "https://{}/callback?address={}&frontend_nonce={}",
        callback_base_url, address, frontend_nonce
    )
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[tokio::test]
//     #[ignore]
//     async fn send_tweet_test() {
//         env_logger::init();
//         dotenv::dotenv().ok();
//         let tweet_text = "Wow!".to_string();
//         let access_token = std::env::var("TEST_ACCESS_TOKEN")
//             .expect("TEST_ACCESS_TOKEN not set")
//             .to_string();
//         let access_secret = std::env::var("TEST_ACCESS_SECRET")
//             .expect("TEST_ACCESS_SECRET not set")
//             .to_string();
//         let _ = send_tweet(access_token, access_secret, tweet_text, None).await;
//     }

//     #[tokio::test]
//     // #[ignore]
//     async fn like_tweet_test() {
//         env_logger::init();
//         dotenv::dotenv().ok();
//         let access_token = std::env::var("TEST_ACCESS_TOKEN")
//             .expect("TEST_ACCESS_TOKEN not set")
//             .to_string();
//         let access_secret = std::env::var("TEST_ACCESS_SECRET")
//             .expect("TEST_ACCESS_SECRET not set")
//             .to_string();
//         let x_info = get_user_x_info(access_token.clone(), access_secret.clone()).await;
//         let x_id = x_info.id;
//         let _ = like_tweet(
//             access_token,
//             access_secret,
//             x_id,
//             "1803455775911694374".to_string(),
//         )
//         .await;
//     }

//     #[tokio::test]
//     async fn get_user_info_test() {
//         env_logger::init();
//         dotenv::dotenv().ok();
//         let access_token = std::env::var("TEST_ACCESS_TOKEN")
//             .expect("TEST_ACCESS_TOKEN not set")
//             .to_string();
//         let access_secret = std::env::var("TEST_ACCESS_SECRET")
//             .expect("TEST_ACCESS_SECRET not set")
//             .to_string();
//         get_user_x_info(access_token, access_secret).await;
//     }
// }
