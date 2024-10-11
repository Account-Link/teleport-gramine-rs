use std::{str::FromStr, sync::Arc};

use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder, WsConnect},
    pubsub::SubscriptionStream,
    rpc::types::{BlockNumberOrTag, Filter, Log},
    sol_types::SolEventInterface,
};
use eyre::Result;
use futures_util::stream::StreamExt;
use tokio::sync::Mutex;

type LogStream = SubscriptionStream<Log>;

use super::{handle_event, handlers::HandlerContext, NFTEvents};
use crate::{
    db::{client_db::ClientDB, TeleportDB},
    oai::OpenAIClient,
    twitter::builder::TwitterBuilder,
};

pub async fn subscribe_to_nft_events<A: TeleportDB>(
    db: Arc<Mutex<A>>,
    twitter_builder: Arc<TwitterBuilder>,
    ws_rpc_url: String,
    client_db_url: String,
    openai_client: Arc<OpenAIClient>,
    nft_address: String,
) -> Result<()> {
    let mut stream = nft_event_stream(&ws_rpc_url, &nft_address).await?;

    let client_db = ClientDB::new(client_db_url);

    while let Some(log) = stream.next().await {
        if let Ok(event) = NFTEvents::decode_raw_log(log.topics(), &log.data().data, true) {
            let context = HandlerContext {
                db: db.clone(),
                client_db: client_db.clone(),
                twitter_builder: twitter_builder.clone(),
                openai_client: openai_client.clone(),
            };

            tokio::spawn(async move {
                if let Err(e) = handle_event(&context, log.transaction_hash, event).await {
                    log::error!("Error handling event: {:?}", e);
                }
            });
        }
    }

    Ok(())
}

async fn nft_event_stream(ws_rpc_url: &str, nft_address: &str) -> Result<LogStream, eyre::Error> {
    let ws = WsConnect::new(ws_rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await?;
    let filter =
        Filter::new().address(Address::from_str(nft_address)?).from_block(BlockNumberOrTag::Latest);
    let sub = provider.subscribe_logs(&filter).await?;
    log::info!("Subscribed to events for contract at: {}", nft_address);
    let stream = sub.into_stream();
    Ok(stream)
}
