use std::sync::Arc;

use alloy::{hex::ToHexExt, primitives::FixedBytes};
use eyre::{OptionExt, Result};
use tokio::sync::Mutex;

// New struct to hold common handler context
pub struct HandlerContext<A: TeleportDB> {
    pub db: Arc<Mutex<A>>,
    pub client_db: ClientDB,
    pub twitter_builder: Arc<TwitterBuilder>,
    pub openai_client: Arc<OpenAIClient>,
}

use super::{
    contract::{NFTEvents, NewTokenData, RedeemTweet, Transfer},
    TweetContent,
};
use crate::{
    db::{client_db::ClientDB, TeleportDB},
    oai::OpenAIClient,
    twitter::{builder::TwitterBuilder, tweet::Tweet},
};

/// Handles various on-chain NFT-related events
///
/// This function processes events emitted by the NFT smart contract It delegates to specific
/// handler functions based on the event type, managing RedeemTweet, NewTokenData, and Transfer
/// events as they occur on the blockchain.
///
/// # Arguments
///
/// * `db` - Shared database access for TeleportDB operations
/// * `client_db` - frontend DB
/// * `twitter_builder` - Factory for creating authenticated Twitter API clients
/// * `tx_hash` - Optional transaction hash of the on-chain event
/// * `event` - The specific on-chain event to handle
/// * `openai_client` - Shared OpenAI client for content moderation and safety checks
///
/// # Returns
///
/// Returns a Result indicating success or failure of the on-chain event handling
pub async fn handle_event<A: TeleportDB>(
    context: &HandlerContext<A>,
    tx_hash: Option<FixedBytes<32>>,
    event: NFTEvents,
) -> Result<()> {
    let result = match &event {
        NFTEvents::RedeemTweet(redeem) => handle_redeem_tweet(context, redeem).await,
        NFTEvents::NewTokenData(new_token_data) => {
            handle_new_token_data(context, tx_hash, new_token_data).await
        }
        NFTEvents::Transfer(transfer) => handle_transfer(context, transfer).await,
        _ => Ok(()),
    };

    if let Err(e) = &result {
        log::error!("Error handling event {:?}: {:?}", event, e);
    }

    result
}

/// Handles the RedeemTweet event by posting a tweet and updating relevant databases.
///
/// This function checks if the tweet content is safe, posts the tweet, and updates the database
/// with the redeemed tweet information.
///
/// # Arguments
///
/// * `context` - Handler context containing shared database and Twitter API access
/// * `redeem` - Event data containing tweet content and redemption details
///
/// # Returns
///
/// Returns a Result indicating success or failure of the on-chain event handling
async fn handle_redeem_tweet<A: TeleportDB>(
    context: &HandlerContext<A>,
    redeem: &RedeemTweet,
) -> Result<()> {
    let safe = context.openai_client.is_tweet_safe(&redeem.content, &redeem.policy).await?;
    if safe {
        let db_lock = context.db.lock().await;
        let user = db_lock.get_user_by_x_id(redeem.x_id.to_string()).ok();
        drop(db_lock);
        let mut tweet_content = TweetContent { text: redeem.content.clone(), media_url: None };

        if let Some(user) = user {
            let client = context
                .twitter_builder
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

            let mut db = context.db.lock().await;
            db.add_tweet(redeem.tokenId.to_string(), tweet_id)?;
            drop(db);
        }

        let token_id = redeem.tokenId.to_string();
        let token_owner = context.client_db.get_token_owner(token_id.clone()).await?;
        context
            .client_db
            .add_redeemed_tweet(
                token_owner.clone(),
                token_id.clone(),
                tweet_content.text,
                redeem.policy.clone(),
            )
            .await?;
        context.client_db.increment_user_redeemed(token_owner.user_id.clone()).await?;
        context.client_db.delete_token(token_id).await?;
        log::info!("NFT {} deleted on postgresdb.", redeem.tokenId.to_string());
    }
    Ok(())
}

/// Handles the NewTokenData event, which involves promoting a pending NFT and updating the
/// database.
///
/// This function promotes a pending NFT to a full NFT status in the database and logs the minting.
///
/// # Arguments
///
/// * `context` - Handler context containing shared database and Twitter API access
/// * `transaction_hash` - Optional transaction hash associated with the new token
/// * `new_token_data` - NewTokenData event information
///
/// # Returns
///
/// Returns a Result indicating success or failure of the new token data handling
async fn handle_new_token_data<A: TeleportDB>(
    context: &HandlerContext<A>,
    transaction_hash: Option<FixedBytes<32>>,
    new_token_data: &NewTokenData,
) -> Result<()> {
    let mut db = context.db.lock().await;
    let nft_id = db.promote_pending_nft(
        transaction_hash.ok_or_eyre("Transaction hash is missing")?.encode_hex_with_prefix(),
        new_token_data.tokenId.to_string(),
    )?;
    drop(db);

    let token_id = new_token_data.tokenId.to_string();
    context.client_db.set_token_id(token_id.clone(), nft_id).await?;
    log::info!(
        "NFT minted with id {} to address {}",
        new_token_data.tokenId.to_string(),
        new_token_data.to.to_string()
    );
    Ok(())
}

/// Handles the Transfer event, which involves updating token ownership in the database.
///
/// This function updates the database to reflect changes in token ownership, including
/// minting (transfer from zero address) and burning (transfer to zero address) operations.
///
/// # Arguments
///
/// * `client_db` - frontend DB instance for database operations
/// * `transfer` - Transfer event information
///
/// # Returns
///
/// Returns a Result indicating success or failure of the transfer handling
async fn handle_transfer<A: TeleportDB>(
    context: &HandlerContext<A>,
    transfer: &Transfer,
) -> Result<()> {
    let from = transfer.from.to_string();
    let to = transfer.to.to_string();
    let token_id = transfer.tokenId.to_string();
    let client_db = &context.client_db;

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
