use std::{str::FromStr, sync::Arc};

use alloy::{
    hex::ToHexExt,
    network::TransactionBuilder,
    primitives::{address, utils::parse_units, Address, Uint},
    providers::{network::EthereumWallet, Provider, ProviderBuilder, WsConnect},
    rpc::types::{BlockNumberOrTag, Filter, TransactionRequest},
    sol,
    sol_types::SolEventInterface,
};
use futures_util::stream::StreamExt;
use tokio::sync::Mutex;
use NFT::NFTEvents;

use super::wallet::WalletProvider;
use crate::{db::TeleportDB, oai, twitter::send_tweet};

sol!(
    #[sol(rpc)]
    NFT,
    "abi/nft.json"
);

pub const NFT_ADDRESS: Address = address!("Fab6a41b917f5e591B4A49736568Ac7845cb0245");

pub async fn subscribe_to_nft_events<A: TeleportDB>(
    db: Arc<Mutex<A>>,
    ws_rpc_url: String,
) -> eyre::Result<()> {
    let ws = WsConnect::new(ws_rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    let filter = Filter::new()
        .address(NFT_ADDRESS)
        .from_block(BlockNumberOrTag::Latest);

    log::info!(
        "Subscribed to events for contract at: {}",
        NFT_ADDRESS.to_string()
    );

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        if let Ok(event) = NFTEvents::decode_raw_log(log.topics(), &log.data().data, true) {
            match event {
                NFTEvents::Redeem(redeem) => {
                    let safe = oai::is_tweet_safe(&redeem.content, &redeem.policy).await;
                    if safe {
                        let db_lock = db.lock().await;
                        let user = db_lock.get_user_by_x_id(redeem.x_id.to_string()).await.ok();
                        drop(db_lock);
                        if let Some(user) = user {
                            let tweet_id = send_tweet(
                                user.access_token,
                                user.access_secret,
                                redeem.content.to_string(),
                            )
                            .await?;

                            let mut db = db.lock().await;
                            db.add_tweet(redeem.tokenId.to_string(), tweet_id).await?;
                            drop(db);
                        }
                    }
                }
                NFTEvents::NewTokenData(new_token_data) => {
                    let mut db = db.lock().await;
                    db.promote_pending_nft(
                        log.transaction_hash.unwrap().encode_hex_with_prefix(),
                        new_token_data.tokenId.to_string(),
                    )
                    .await?;
                    drop(db);
                    log::info!(
                        "NFT minted with id {} to address {}",
                        new_token_data.tokenId.to_string(),
                        new_token_data.to.to_string()
                    );
                }
                _ => continue,
            }
        }
    }

    Ok(())
}

pub async fn mint_nft(
    provider: WalletProvider,
    recipient: Address,
    x_id: String,
    policy: String,
) -> eyre::Result<String> {
    let nft = NFT::new(NFT_ADDRESS, provider);
    let mint = nft.mintTo(recipient, Uint::from_str(&x_id)?, policy);
    let tx = mint.send().await?;

    let tx_hash = tx.tx_hash();

    log::info!("Minted NFT with tx hash: {}", tx_hash);

    Ok(tx_hash.encode_hex_with_prefix())
}

pub async fn redeem_nft(
    wallet: EthereumWallet,
    rpc_url: String,
    token_id: String,
    content: String,
) -> eyre::Result<String> {
    let rpc_url = rpc_url.parse()?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);

    let nft = NFT::new(NFT_ADDRESS, provider);
    let token_id = Uint::from_str(&token_id)?;
    let redeem = nft.redeem(token_id, content);
    let tx = redeem.send().await?;

    let tx_hash = tx.tx_hash();

    log::info!("Redeemed NFT with tx hash: {}", tx_hash);
    Ok(tx_hash.encode_hex_with_prefix())
}

pub async fn send_eth(
    provider: WalletProvider,
    recipient: Address,
    amount: &str,
) -> eyre::Result<()> {
    let tx = TransactionRequest::default()
        .with_to(recipient)
        .with_value(parse_units(amount, "ether").unwrap().into());
    let _ = provider.send_transaction(tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy::signers::local::{coins_bip39::English, MnemonicBuilder};

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
        mint_nft(
            provider,
            recipient_address,
            1.to_string(),
            "policy".to_string(),
        )
        .await
        .unwrap();
    }
}
