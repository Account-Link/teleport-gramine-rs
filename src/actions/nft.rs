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
    db::{client_db::ClientDB, TeleportDB},
    oai,
    twitter::{builder::TwitterBuilder, tweet::Tweet},
};

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
    database_url: String,
) -> eyre::Result<()> {
    let ws = WsConnect::new(ws_rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await?;
    let nft_address = get_nft_address()?;

    let filter = Filter::new().address(nft_address).from_block(BlockNumberOrTag::Latest);

    log::info!("Subscribed to events for contract at: {}", nft_address.to_string());

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    let client_db = ClientDB::new(database_url);

    while let Some(log) = stream.next().await {
        if let Ok(event) = NFTEvents::decode_raw_log(log.topics(), &log.data().data, true) {
            let db = db.clone();
            let twitter_builder = twitter_builder.clone();
            let client_db = client_db.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    handle_event(db, client_db, twitter_builder, log.transaction_hash, event).await
                {
                    log::error!("Error handling event: {:?}", e);
                }
            });
        }
    }

    Ok(())
}

async fn handle_event<A: TeleportDB>(
    db: Arc<Mutex<A>>,
    client_db: ClientDB,
    twitter_builder: TwitterBuilder,
    tx_hash: Option<FixedBytes<32>>,
    event: NFTEvents,
) -> eyre::Result<()> {
    match event {
        NFTEvents::RedeemTweet(redeem) => {
            if let Err(e) = handle_redeem_tweet(db, client_db, twitter_builder, redeem).await {
                log::error!("Error handling RedeemTweet event: {:?}", e);
            }
        }
        NFTEvents::NewTokenData(new_token_data) => {
            if let Err(e) = handle_new_token_data(db, client_db, tx_hash, new_token_data).await {
                log::error!("Error handling NewTokenData event: {:?}", e);
            }
        }
        NFTEvents::Transfer(transfer) => {
            if let Err(e) = handle_transfer(client_db, transfer).await {
                log::error!("Error handling Transfer event: {:?}", e);
            }
        }
        _ => {}
    };
    Ok(())
}

async fn handle_redeem_tweet<A: TeleportDB>(
    db: Arc<Mutex<A>>,
    client_db: ClientDB,
    twitter_builder: TwitterBuilder,
    redeem: RedeemTweet,
) -> eyre::Result<()> {
    let safe = oai::is_tweet_safe(&redeem.content, &redeem.policy).await;
    if safe {
        let db_lock = db.lock().await;
        let user = db_lock.get_user_by_x_id(redeem.x_id.to_string()).ok();
        drop(db_lock);
        let mut tweet_content = TweetContent { text: redeem.content.clone(), media_url: None };

        if let Some(user) = user {
            let client = twitter_builder
                .with_auth(user.access_tokens.ok_or_eyre("User has no access tokens")?.into());

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

            let mut tweet = Tweet::new(tweet_content.text.clone());
            if let Some(media_id) = tweet_content.media_url {
                tweet.set_media_ids(vec![media_id]);
            }

            let tweet_id = client.raw_tweet(tweet).await?;

            let mut db = db.lock().await;
            db.add_tweet(redeem.tokenId.to_string(), tweet_id)?;
            drop(db);
        }

        let token_id = redeem.tokenId.to_string();
        let token_owner = client_db.get_token_owner(token_id.clone()).await?;
        client_db
            .add_redeemed_tweet(
                token_owner.clone(),
                token_id.clone(),
                tweet_content.text,
                redeem.policy,
            )
            .await?;
        client_db.increment_user_redeemed(token_owner.user_id).await?;
        client_db.delete_token(token_id).await?;
        log::info!("NFT {} deleted on postgresdb.", redeem.tokenId.to_string());
    }
    Ok(())
}

async fn handle_new_token_data<A: TeleportDB>(
    db: Arc<Mutex<A>>,
    client_db: ClientDB,
    transaction_hash: Option<FixedBytes<32>>,
    new_token_data: NewTokenData,
) -> eyre::Result<()> {
    let mut db = db.lock().await;
    let nft_id = db.promote_pending_nft(
        transaction_hash.ok_or_eyre("Transaction hash is missing")?.encode_hex_with_prefix(),
        new_token_data.tokenId.to_string(),
    )?;
    drop(db);

    let token_id = new_token_data.tokenId.to_string();
    client_db.set_token_id(token_id.clone(), nft_id).await?;
    log::info!(
        "NFT minted with id {} to address {}",
        new_token_data.tokenId.to_string(),
        new_token_data.to.to_string()
    );
    Ok(())
}

async fn handle_transfer(client_db: ClientDB, transfer: Transfer) -> eyre::Result<()> {
    let from = transfer.from.to_string();
    let to = transfer.to.to_string();
    let token_id = transfer.tokenId.to_string();

    if from == "0x0000000000000000000000000000000000000000" {
        // Do nothing
    } else if to == "0x0000000000000000000000000000000000000000" {
        client_db.delete_token(token_id.clone()).await?;
    } else {
        client_db.update_token_owner(token_id.clone(), to.clone()).await?;
    }

    log::info!("NFT {} transferred from {} to {}.", token_id, from, to);
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

    use crate::actions::wallet::get_provider;

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
        let provider = get_provider(rpc_url, wallet);
        mint_nft(provider, recipient_address, 1.to_string(), "policy".to_string()).await.unwrap();
    }
}
