/// Services for interacting with NFT contracts.
use std::str::FromStr;

use alloy::{
    hex::ToHexExt,
    primitives::{Address, Uint},
};
use eyre::Result;

use super::contract::NFT;
use crate::actions::wallet::WalletProvider;

/// Mints a new NFT to the specified recipient.
///
/// # Arguments
///
/// * `provider` - The wallet provider for transaction signing.
/// * `recipient` - The address of the NFT recipient.
/// * `x_id` - The X ID of the user.
/// * `policy` - The policy associated with the NFT.
/// * `nft_address` - The address of the NFT contract.
///
/// # Returns
///
/// The transaction hash as a hexadecimal string.
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

/// Redeems an NFT with the specified token ID and content.
///
/// # Arguments
///
/// * `provider` - The wallet provider for transaction signing.
/// * `token_id` - The ID of the token to be redeemed.
/// * `content` - The content associated with the redemption.
/// * `nft_address` - The address of the NFT contract.
///
/// # Returns
///
/// The transaction hash as a hexadecimal string.
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
