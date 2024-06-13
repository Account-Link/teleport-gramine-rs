use alloy::primitives::address;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy::providers::WsConnect;
use alloy::rpc::types::BlockNumberOrTag;
use alloy::rpc::types::Filter;
use alloy_sol_types::sol;
use alloy_sol_types::SolEventInterface;
use futures_util::stream::StreamExt;
use NFT::NFTEvents;

use crate::db;
use crate::oai;
use crate::twitter::send_tweet;

sol!(NFT, "src/abi.json");

pub async fn subscribe_to_events(db_url: String, ws_rpc_url: String) -> eyre::Result<()> {
    let ws = WsConnect::new(ws_rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    let nft_address = address!("3154Cf16ccdb4C6d922629664174b904d80F2C35");
    let filter = Filter::new()
        .address(nft_address)
        .from_block(BlockNumberOrTag::Latest);

    log::info!(
        "Subscribed to events for contract at: {}",
        nft_address.to_string()
    );

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        if let Ok(event) = NFTEvents::decode_raw_log(log.topics(), &log.data().data, true) {
            match event {
                NFTEvents::Redeem(redeem) => {
                    let safe = oai::is_tweet_safe(&redeem.content, &redeem.policy).await;
                    if safe {
                        let x_id = redeem.x_id.into_limbs()[0];
                        let tokens = db::get_access_tokens(db_url.clone(), x_id).await.ok();
                        if let Some((access_token, access_secret)) = tokens {
                            send_tweet(access_token, access_secret, redeem.content.to_string())
                                .await;
                        }
                    }
                }
                _ => continue,
            }
        }
    }

    Ok(())
}
