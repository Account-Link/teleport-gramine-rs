use std::str::FromStr;

use alloy::{
    network::EthereumWallet,
    primitives::{Address, Uint},
    providers::{
        fillers::{
            BlobGasFiller, CachedNonceManager, ChainIdFiller, GasFiller, JoinFill, NonceFiller,
        },
        ProviderBuilder,
    },
    signers::local::{coins_bip39::English, MnemonicBuilder},
    sol,
};
use serde::Serialize;

sol!(
    #[sol(rpc)]
    Redeem,
    "abi/redeem.json"
);

#[derive(Serialize)]
struct TweetContent {
    text: String,
    media_url: Option<String>,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let mnemonic = std::env::var("NFT_MINTER_MNEMONIC").expect("NFT_MINTER_MNEMONIC not set");
    let rpc_key = std::env::var("RPC_KEY").expect("RPC_KEY not set");
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL not set");
    let rpc_url = rpc_url + &rpc_key;

    let signer =
        MnemonicBuilder::<English>::default().phrase(mnemonic).index(0).unwrap().build().unwrap();
    let wallet: EthereumWallet = signer.into();

    let provider = ProviderBuilder::new()
        .filler(JoinFill::new(
            GasFiller,
            JoinFill::new(
                BlobGasFiller,
                JoinFill::new(
                    NonceFiller::<CachedNonceManager>::default(),
                    ChainIdFiller::default(),
                ),
            ),
        ))
        .wallet(wallet)
        .on_http(rpc_url.parse().unwrap());

    let nft = Redeem::new(
        Address::from_str("0x0b33bd59FCa63390A341ee6f608Bf5Ed1393ffcc").unwrap(),
        provider,
    );

    let tweet_content = TweetContent {
        text: "libmev mevbot take 2: batchredeem with image".to_string(),
        media_url: Some("https://i.imgur.com/HLHBnl9.jpeg".to_string()),
    };

    let content = serde_json::to_string(&tweet_content).unwrap();

    let token_ids = (244..340).map(|i| Uint::from(i)).collect::<Vec<_>>();

    let redeem = nft.redeem(token_ids, content);
    let tx = redeem.send().await.unwrap();
    let tx_hash = tx.tx_hash();
    log::info!("Redeemed NFT with tx hash: {}", tx_hash);
}
