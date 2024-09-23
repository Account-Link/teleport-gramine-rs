use serde::{Deserialize, Serialize};

use super::builder::TwitterClient;

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

impl TwitterClient<'_> {
    pub async fn get_user_info(&self) -> eyre::Result<UserInfo> {
        let resp = self.client
            .get(
            "https://api.twitter.com/2/users/me?user.fields=profile_image_url,most_recent_tweet_id"
                .to_string(),
        )
        .send()
        .await?;
        let user_info: UserInfoResponse = resp.json().await?;
        let user_info = user_info.data;
        log::info!("Fetched x_info: {:?}", user_info);
        Ok(user_info)
    }
}
