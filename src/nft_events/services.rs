use std::str::FromStr;

use alloy::{
    hex::ToHexExt,
    primitives::{Address, Uint},
};
use eyre::Result;

use super::contract::NFT;
use crate::actions::wallet::WalletProvider;

pub async fn mint_nft(
    provider: WalletProvider,
    recipient: Address,
    x_id: String,
    policy: String,
    nft_address: &str,
) -> Result<String> {
    let nft_address = Address::from_str(nft_address)?;
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
    nft_address: &str,
) -> Result<String> {
    let nft_address = Address::from_str(nft_address)?;
    let nft = NFT::new(nft_address, provider);
    let token_id = Uint::from_str(&token_id)?;
    let redeem = nft.redeem(token_id, content, 0u8);
    let tx = redeem.send().await?;

    let tx_hash = tx.tx_hash();

    log::info!("Redeemed NFT with tx hash: {}", tx_hash);
    Ok(tx_hash.encode_hex_with_prefix())
}
