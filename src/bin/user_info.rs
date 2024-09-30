use std::{
    fs::File,
    io::BufReader,
    sync::{Arc, Mutex},
};

use rayon::prelude::*;
use teleport::twitter::{auth::TwitterTokenPair, builder::TwitterBuilder};

#[derive(serde::Deserialize)]
struct Tokens {
    tokens: Vec<TwitterTokenPair>,
}

#[derive(serde::Serialize)]
struct UserInfo {
    username: String,
    x_id: String,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let app_key = std::env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set");
    let app_secret =
        std::env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set");
    let twitter = TwitterBuilder::new(app_key, app_secret);

    let file = File::open("tokens.json").unwrap();
    let reader = BufReader::new(file);
    let tokens: Tokens = serde_json::from_reader(reader).unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let user_infos: Vec<UserInfo> = Vec::new();
    let user_infos = Arc::new(Mutex::new(user_infos));
    tokens.tokens.par_iter().for_each(|token_pair| {
        let client = twitter.with_auth(token_pair.clone());
        let info = runtime.block_on(client.get_user_info()).unwrap();
        let info = UserInfo { username: info.username, x_id: info.id };
        let mut user_infos = user_infos.lock().unwrap();
        user_infos.push(info);
        drop(user_infos);
    });
    drop(runtime);
    let file = File::create("user_infos.json").unwrap();
    let user_infos = user_infos.lock().unwrap();
    let user_infos = user_infos.as_slice();
    serde_json::to_writer_pretty(file, user_infos).unwrap();
}
