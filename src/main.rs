use std::str::FromStr;

use alloy_sol_types::{sol, SolEventInterface};
use reth_exex::{ExExContext, ExExEvent};
use reth_node_api::FullNodeComponents;
use reth_node_ethereum::EthereumNode;
use reth_primitives::{Address, Log, SealedBlockWithSenders, TransactionSigned};
use reth_provider::Chain;
mod oai;
mod twitter;

sol!(NFT, "src/abi.json");
use twitter::send_tweet;
use NFT::NFTEvents;

async fn teleport_exex<Node: FullNodeComponents>(mut ctx: ExExContext<Node>) -> eyre::Result<()> {
    while let Some(notification) = ctx.notifications.recv().await {
        if let Some(committed_chain) = notification.committed_chain() {
            let events = decode_chain_into_events(&committed_chain);

            for (_block, _tx, _log, event) in events {
                match event {
                    NFTEvents::Redeem(redeem) => {
                        let safe = oai::is_tweet_safe(&redeem.content, &redeem.policy).await;
                        if safe {
                            //todo: get access token + secret
                            send_tweet("".to_string(), "".to_string(), redeem.content.to_string())
                                .await;
                        }
                    }
                    _ => continue,
                };
            }

            ctx.events
                .send(ExExEvent::FinishedHeight(committed_chain.tip().number))?;
        }
    }

    Ok(())
}

fn decode_chain_into_events(
    chain: &Chain,
) -> impl Iterator<Item = (&SealedBlockWithSenders, &TransactionSigned, &Log, NFTEvents)> {
    chain
        .blocks_and_receipts()
        .flat_map(|(block, receipts)| {
            block
                .body
                .iter()
                .zip(receipts.iter().flatten())
                .map(move |(tx, receipt)| (block, tx, receipt))
        })
        .flat_map(|(block, tx, receipt)| {
            receipt
                .logs
                .iter()
                .filter(|log| {
                    log.address
                        == Address::from_str("0x3154Cf16ccdb4C6d922629664174b904d80F2C35").unwrap()
                })
                .map(move |log| (block, tx, log))
        })
        .filter_map(|(block, tx, log)| {
            NFTEvents::decode_raw_log(log.topics(), &log.data.data, true)
                .ok()
                .map(|event| (block, tx, log, event))
        })
}

fn main() -> eyre::Result<()> {
    reth::cli::Cli::parse_args().run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("Teleport", |ctx| async move { Ok(teleport_exex(ctx)) })
            .launch()
            .await?;

        handle.wait_for_node_exit().await
    })
}
