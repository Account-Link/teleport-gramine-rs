pub mod handlers;
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
