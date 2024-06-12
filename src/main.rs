use std::str::FromStr;

use alloy_sol_types::{sol, SolEventInterface};
use db::create_tables;
use futures::Future;
use reth_exex::{ExExContext, ExExEvent};
use reth_node_api::FullNodeComponents;
use reth_node_ethereum::EthereumNode;
use reth_primitives::{Address, Log, SealedBlockWithSenders, TransactionSigned};
use reth_provider::Chain;
use rusqlite::Connection;
mod db;
mod oai;
mod twitter;

sol!(NFT, "src/abi.json");
use twitter::send_tweet;
use NFT::NFTEvents;

async fn teleport_exex<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
    db_url: String,
) -> eyre::Result<()> {
    while let Some(notification) = ctx.notifications.recv().await {
        if let Some(committed_chain) = notification.committed_chain() {
            let events = decode_chain_into_events(&committed_chain);

            for (_block, _tx, _log, event) in events {
                match event {
                    NFTEvents::Redeem(redeem) => {
                        let safe = oai::is_tweet_safe(&redeem.content, &redeem.policy).await;
                        if safe {
                            let x_id = redeem.x_id.into_limbs()[0];
                            let (access_token, access_secret) =
                                db::get_user_tokens(db_url.clone(), x_id).await?;
                            send_tweet(access_token, access_secret, redeem.content.to_string())
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

async fn init<Node: FullNodeComponents>(
    ctx: ExExContext<Node>,
    db_url: String,
) -> eyre::Result<impl Future<Output = eyre::Result<()>>> {
    let mut connection = Connection::open(db_url.clone())?;
    create_tables(&mut connection)?;
    Ok(teleport_exex(ctx, db_url))
}

fn main() -> eyre::Result<()> {
    reth::cli::Cli::parse_args().run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("Teleport", |ctx| async move {
                let db_url = std::env::var("DB_URL").expect("DB_URL not set");
                init(ctx, db_url).await
            })
            .launch()
            .await?;

        handle.wait_for_node_exit().await
    })
}
