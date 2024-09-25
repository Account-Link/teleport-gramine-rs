use std::{str::FromStr, sync::Arc};

use alloy::{
    hex::ToHexExt,
    primitives::{Address, FixedBytes, Uint},
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::{BlockNumberOrTag, Filter},
    sol,
    sol_types::SolEventInterface,
};
use eyre::OptionExt;
use futures_util::stream::StreamExt;
use serde::Deserialize;
use tokio::sync::Mutex;
use NFT::NFTEvents;

use self::NFT::{NewTokenData, RedeemTweet, Transfer};

use super::wallet::WalletProvider;
use crate::{
    db::TeleportDB,
    oai,
    twitter::{builder::TwitterBuilder, tweet::Tweet},
};
use rustls::ClientConfig;
use tokio_postgres_rustls::MakeRustlsConnect;

sol!(
    #[sol(rpc)]
    NFT,
    "abi/nft.json"
);

#[derive(Deserialize)]
struct TweetContent {
    text: String,
    media_url: Option<String>,
}

pub fn get_nft_address() -> eyre::Result<Address> {
    let nft_address = std::env::var("NFT_ADDRESS")?;
    Ok(Address::from_str(&nft_address)?)
}

pub async fn subscribe_to_nft_events<A: TeleportDB>(
    db: Arc<Mutex<A>>,
    twitter_builder: TwitterBuilder,
    ws_rpc_url: String,
) -> eyre::Result<()> {
    let ws = WsConnect::new(ws_rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await?;
    let nft_address = get_nft_address()?;

    let filter = Filter::new().address(nft_address).from_block(BlockNumberOrTag::Latest);

    log::info!("Subscribed to events for contract at: {}", nft_address.to_string());

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        if let Ok(event) = NFTEvents::decode_raw_log(log.topics(), &log.data().data, true) {
            match event {
                NFTEvents::RedeemTweet(redeem) => {
                    if let Err(e) =
                        handle_redeem_tweet(db.clone(), twitter_builder.clone(), redeem).await
                    {
                        log::error!("Error handling RedeemTweet event: {:?}", e);
                    }
                }
                NFTEvents::NewTokenData(new_token_data) => {
                    if let Err(e) =
                        handle_new_token_data(db.clone(), log.transaction_hash, new_token_data)
                            .await
                    {
                        log::error!("Error handling NewTokenData event: {:?}", e);
                    }
                }
                NFTEvents::Transfer(transfer) => {
                    if let Err(e) = handle_transfer(transfer).await {
                        log::error!("Error handling Transfer event: {:?}", e);
                    }
                }
                _ => continue,
            }
        }
    }

    Ok(())
}

async fn handle_redeem_tweet<A: TeleportDB>(
    db: Arc<Mutex<A>>,
    twitter_builder: TwitterBuilder,
    redeem: RedeemTweet,
) -> eyre::Result<()> {
    let safe = oai::is_tweet_safe(&redeem.content, &redeem.policy).await;
    if safe {
        let db_lock = db.lock().await;
        let user = db_lock.get_user_by_x_id(redeem.x_id.to_string()).await.ok();
        drop(db_lock);
        if let Some(user) = user {
            let client = twitter_builder
                .with_auth(user.access_tokens.ok_or_eyre("User has no access tokens")?.into());

            let mut tweet_content = TweetContent { text: redeem.content.clone(), media_url: None };

            // to be backwards compatible for now
            if let Ok(parsed_tweet_content) = serde_json::from_str::<TweetContent>(&redeem.content)
            {
                tweet_content.text = parsed_tweet_content.text;
                if let Some(media_url) = parsed_tweet_content.media_url {
                    let media_bytes = reqwest::get(media_url).await?.bytes().await?.to_vec();
                    let media_id = client.upload_media(media_bytes, None).await?;
                    tweet_content.media_url = Some(media_id);
                }
            }

            let mut tweet = Tweet::new(tweet_content.text);
            if let Some(media_id) = tweet_content.media_url {
                tweet.set_media_ids(vec![media_id]);
            }

            let tweet_id = client.raw_tweet(tweet).await?;

            let mut db = db.lock().await;
            db.add_tweet(redeem.tokenId.to_string(), tweet_id).await?;
            drop(db);
        }
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let mut config = ClientConfig::new();
        config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let tls = MakeRustlsConnect::new(config);
        let (client, connection) = tokio_postgres::connect(&database_url, tls).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("connection error: {}", e);
            }
        });
        let token_id_int: i32 = redeem.tokenId.to_string().parse()?;

        let row = client
            .query_one(
                "SELECT \"userId\", \"twitterUserName\" FROM \"NftIndex\" WHERE \"tokenId\" = $1",
                &[&token_id_int],
            )
            .await?;
        let creator_user_id: String = row.get(0);
        let twitter_user_name: String = row.get(1);

        let tweet_id = "";
        let safeguard = redeem.policy;
        let content = redeem.content;
        let id = cuid::cuid2();

        client.execute(
            "INSERT INTO \"RedeemedIndex\" (\"id\", \"creatorUserId\", \"tokenId\", \"tweetId\", \"twitterUserName\", \"safeguard\", \"content\") VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[&id, &creator_user_id, &token_id_int, &tweet_id, &twitter_user_name, &safeguard, &content],
        )
        .await?;

        client.execute(
            "UPDATE \"User\" SET \"haveBeenRedeemed\" = \"haveBeenRedeemed\" + 1 WHERE \"id\" = $1",
            &[&creator_user_id],
        ).await?;

        client.execute("DELETE FROM \"NftIndex\" WHERE \"tokenId\" = $1", &[&token_id_int]).await?;

        log::info!("NFT {} deleted on postgresdb.", redeem.tokenId.to_string());
    }
    Ok(())
}

async fn handle_new_token_data<A: TeleportDB>(
    db: Arc<Mutex<A>>,
    transaction_hash: Option<FixedBytes<32>>,
    new_token_data: NewTokenData,
) -> eyre::Result<()> {
    let mut db = db.lock().await;
    db.promote_pending_nft(
        transaction_hash.ok_or_eyre("Transaction hash is missing")?.encode_hex_with_prefix(),
        new_token_data.tokenId.to_string(),
    )
    .await?;
    drop(db);
    log::info!(
        "NFT minted with id {} to address {}",
        new_token_data.tokenId.to_string(),
        new_token_data.to.to_string()
    );
    Ok(())
}

async fn handle_transfer(transfer: Transfer) -> eyre::Result<()> {
    let from = transfer.from.to_string();
    let to = transfer.to.to_string();
    let token_id_int: i32 = transfer.tokenId.to_string().parse()?;

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut config = ClientConfig::new();
    config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    let tls = MakeRustlsConnect::new(config);
    let (client, connection) = tokio_postgres::connect(&database_url, tls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            log::error!("connection error: {}", e);
        }
    });

    if from == "0x0000000000000000000000000000000000000000" {
        // Do nothing
    } else if to == "0x0000000000000000000000000000000000000000" {
        client.execute("DELETE FROM \"NftIndex\" WHERE \"tokenId\" = $1", &[&token_id_int]).await?;
    } else {
        client
            .execute(
                "UPDATE \"NftIndex\" SET \"userId\" = $1 WHERE \"tokenId\" = $2",
                &[&to, &token_id_int],
            )
            .await?;
    }

    log::info!("NFT {} transferred from {} to {}.", token_id_int, from, to);
    Ok(())
}

pub async fn mint_nft(
    provider: WalletProvider,
    recipient: Address,
    x_id: String,
    policy: String,
) -> eyre::Result<String> {
    let nft_address = get_nft_address()?;
    let nft = NFT::new(nft_address, provider);
    let mint = nft.mintTo(recipient, Uint::from_str(&x_id)?, policy);
    let tx = mint.send().await?;

    let tx_hash = tx.tx_hash();

    log::info!("Minted NFT with tx hash: {}", tx_hash);

    Ok(tx_hash.encode_hex_with_prefix())
}

pub async fn redeem_nft(
    provider: WalletProvider,
    token_id: String,
    content: String,
) -> eyre::Result<String> {
    let nft_address = get_nft_address()?;
    let nft = NFT::new(nft_address, provider);
    let token_id = Uint::from_str(&token_id)?;
    let redeem = nft.redeem(token_id, content, 0u8);
    let tx = redeem.send().await?;

    let tx_hash = tx.tx_hash();

    log::info!("Redeemed NFT with tx hash: {}", tx_hash);
    Ok(tx_hash.encode_hex_with_prefix())
}

// pub async fn send_eth(
//     provider: WalletProvider,
//     recipient: Address,
//     amount: &str,
// ) -> eyre::Result<()> {
//     let tx = TransactionRequest::default()
//         .with_to(recipient)
//         .with_value(parse_units(amount, "ether").unwrap().into());
//     let _ = provider.send_transaction(tx).await?;
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use alloy::{
        network::EthereumWallet,
        primitives::address,
        signers::local::{coins_bip39::English, MnemonicBuilder},
    };

    use super::*;
    #[tokio::test]
    async fn test_mint_nft() {
        env_logger::init();
        dotenv::dotenv().ok();
        let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set");
        let recipient_address = address!("36e7Fda8CC503D5Ec7729A42eb86EF02Af315Bf9");
        let mnemonic =
            std::env::var("NFT_MINTER_MNEMONIC").expect("NFT_MINTER_MNEMONIC must be set");

        let signer = MnemonicBuilder::<English>::default()
            .phrase(mnemonic)
            .index(0)
            .unwrap()
            .build()
            .unwrap();
        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(rpc_url.parse().unwrap());
        mint_nft(provider, recipient_address, 1.to_string(), "policy".to_string()).await.unwrap();
    }
}
