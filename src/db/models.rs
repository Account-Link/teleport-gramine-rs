use rusqlite_from_row::FromRow;
use serde::{Deserialize, Serialize};

use crate::twitter::auth::TwitterTokenPair;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct AccessTokens {
    pub token: String,
    pub secret: String,
}

impl From<TwitterTokenPair> for AccessTokens {
    fn from(token_pair: TwitterTokenPair) -> Self {
        Self { token: token_pair.token, secret: token_pair.secret }
    }
}

impl From<AccessTokens> for TwitterTokenPair {
    fn from(access_tokens: AccessTokens) -> Self {
        Self { token: access_tokens.token, secret: access_tokens.secret }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct User {
    pub x_id: Option<String>,
    pub access_tokens: Option<AccessTokens>,
    pub oauth_tokens: AccessTokens,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, PartialEq, Eq)]
pub struct Nft {
    pub address: String,
    pub token_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, PartialEq, Eq)]
pub struct PendingNFT {
    pub address: String,
    pub nft_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, PartialEq, Eq)]
pub struct Session {
    pub x_id: String,
    pub address: String,
}
