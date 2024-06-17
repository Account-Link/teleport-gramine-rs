use std::{str::FromStr, sync::Arc};

use alloy::{
    hex::ToHexExt,
    primitives::{address, Address, Uint},
    providers::{network::EthereumWallet, Provider, ProviderBuilder, WsConnect},
    rpc::types::{BlockNumberOrTag, Filter},
    sol,
    sol_types::SolEventInterface,
};
use futures_util::stream::StreamExt;
use tokio::sync::Mutex;
use NFT::NFTEvents;

use crate::{db::UserDB, oai, twitter::send_tweet};

sol!(
    #[sol(rpc)]
    NFT,
    "src/abi.json"
);

pub const NFT_ADDRESS: Address = address!("614e72B7d713feB6c682c372E330366af713c577");

pub async fn subscribe_to_nft_events<A: UserDB>(
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
                        // let user = db::get_user_by_x_id(db, redeem.x_id.to_string()).await.ok();
                        let db = db.lock().await;
                        let user = db.get_user_by_x_id(redeem.x_id.to_string()).await.ok();
                        drop(db);
                        if let Some(user) = user {
                            send_tweet(
                                user.access_token,
                                user.access_secret,
                                redeem.content.to_string(),
                            )
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

pub async fn mint_nft(
    wallet: EthereumWallet,
    rpc_url: String,
    recipient: String,
    x_id: String,
    policy: String,
) -> eyre::Result<String> {
    let rpc_url = rpc_url.parse()?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);

    let nft = NFT::new(NFT_ADDRESS, provider);
    let recipient = Address::from_str(&recipient)?;
    let mint = nft.mintTo(recipient, Uint::from_str(&x_id)?, policy, 2);
    let tx = mint.send().await.unwrap();

    let tx_hash = tx.tx_hash();

    log::info!("Minted NFT with tx hash: {}", tx_hash);

    Ok(tx_hash.encode_hex_with_prefix())
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
        mint_nft(
            wallet,
            rpc_url,
            recipient_address.to_string(),
            1.to_string(),
            "policy".to_string(),
        )
        .await
        .unwrap();
    }
}
