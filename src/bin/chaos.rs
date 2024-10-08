use std::{
    fs::File,
    io::{BufReader, Read},
};

use rayon::prelude::*;
use teleport::twitter::{auth::TwitterTokenPair, builder::TwitterBuilder, tweet::Tweet};

#[derive(serde::Deserialize)]
struct Tokens {
    tokens: Vec<TwitterTokenPair>,
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

    let file = File::open("data/test.jpg").unwrap();
    let img_buffer = BufReader::new(file);
    let mut img = Vec::new();
    img_buffer.into_inner().read_to_end(&mut img).unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    tokens.tokens.par_iter().for_each(|token_pair| {
        let client = twitter.with_auth(token_pair.clone());

        let media_id = runtime.block_on(client.upload_media(img.clone(), None)).unwrap();

        let mut tweet = Tweet::new("libmev takeover 3 :o".to_string());
        tweet.set_media_ids(vec![media_id]);
        let res = runtime.block_on(client.raw_tweet(tweet));
        if let Err(e) = res {
            log::error!("Error sending tweet: {:?}", e);
        }
    });
    drop(runtime);
}
