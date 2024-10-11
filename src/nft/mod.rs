//! # NFT Module
//!
//! This module handles NFT-related functionality, including services, event handling, and contract
//! interactions.
//!
//! ## Structure
//!
//! The module is organized into several submodules:
//!
//! - `services`: Provides services for minting and redeeming NFTs.
//! - `handlers`: Contains event handling logic for different NFT events.
//! - `subscription`: Manages the subscription to NFT events.
//! - `contract`: Defines NFT contract types and related functions.
//!
//! ## Key Components
//!
//! ### NFT Services
//!
//! The `services` submodule provides functions for interacting with the NFT contract:
//!
//! - `mint_nft`: Mints a new NFT.
//! - `redeem_nft`: Redeems an NFT.
//!
//! ### Event Handling
//!
//! The `handle_event` function in the `handlers` submodule is the main entry point for processing
//! NFT events. It delegates to specific handlers based on the event type.
//!
//! ### Event Subscription
//!
//! The `subscribe_to_nft_events` function in the `subscription` submodule sets up a WebSocket
//! connection to listen for NFT events.
//!
//! ### Contract Types
//!
//! The `contract` submodule defines the NFT contract types using the `alloy::sol` macro.
//!
//! ## Usage
//!
//! This module is designed to be used internally within the project. It provides the necessary
//! functionality to interact with NFT contracts, handle events, and manage the lifecycle of NFTs
//! within the application.
//!
//! When working with this module, be aware of the following:
//!
//! 1. Event handling is asynchronous and spawned in separate tasks.
//! 2. The module interacts with both a main database and a client database.
//! 3. It integrates with external services like Twitter and OpenAI for content management.
//!
//! Developers should familiarize themselves with the `HandlerContext` struct, which provides
//! shared resources for event handling.
//!
//! ## Note on Safety
//!
//! The module includes content moderation checks using OpenAI before posting tweets. Ensure
//! that the OpenAI client is properly configured and functioning to maintain content safety.

mod handlers;
pub mod services;
mod subscription;

pub use handlers::handle_event;
use serde::Deserialize;
pub use subscription::subscribe_to_nft_events;

/// NFT contract types and related functions
pub mod contract {
    use alloy::sol;

    sol!(
        #[derive(Debug)]
        #[sol(rpc)]
        NFT,
        "abi/nft.json"
    );

    pub use self::NFT::*;
}

// application related models
#[derive(Deserialize)]
pub struct TweetContent {
    pub text: String,
    pub media_url: Option<String>,
}

// Re-export commonly used types for convenience
pub use contract::*;
