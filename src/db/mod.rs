pub mod client_db;
pub mod in_memory;
pub mod models;
pub mod utils;
// pub mod sqlite;

pub use models::*;

pub trait TeleportDB: Send + Sync + 'static {
    // async fn init(&mut self) -> eyre::Result<()>;
    // async fn open_from_file(file_path: &str) -> eyre::Result<Self>;

    /// Adds a new user to the database.
    ///
    /// # Arguments
    ///
    /// * `address` - The user's EVM address.
    /// * `user` - The User struct containing user information.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the user was successfully added, or an error if the operation failed.
    // TODO: Replace the Ethereum address type with a stronger typed EVM address type
    fn add_user(&mut self, address: String, user: User) -> eyre::Result<()>;

    /// Retrieves a user from the database by their EVM address.
    ///
    /// # Arguments
    ///
    /// * `address` - The user's EVM address.
    ///
    /// # Returns
    ///
    /// Returns the User struct if found, or an error if the user doesn't exist.
    fn get_user_by_address(&self, address: String) -> eyre::Result<User>;

    /// Retrieves a user from the database by their Twitter (X) ID.
    ///
    /// # Arguments
    ///
    /// * `x_id` - The user's Twitter (X) ID.
    ///
    /// # Returns
    ///
    /// Returns the User struct if found, or an error if the user doesn't exist.
    fn get_user_by_x_id(&self, x_id: String) -> eyre::Result<User>;

    /// Adds a pending NFT to the database.
    ///
    /// # Arguments
    ///
    /// * `tx_hash` - The transaction hash of the pending NFT.
    /// * `pending_nft` - The PendingNFT struct containing NFT information.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the pending NFT was successfully added, or an error if the operation
    /// failed.
    // TODO: Replace the transaction hash type with a stronger typed transaction hash type
    fn add_pending_nft(&mut self, tx_hash: String, pending_nft: PendingNFT) -> eyre::Result<()>;

    /// Promotes a pending NFT to a minted NFT.
    ///
    /// # Arguments
    ///
    /// * `tx_hash` - The transaction hash of the pending NFT.
    /// * `token_id` - The token ID of the minted NFT.
    ///
    /// # Returns
    ///
    /// Returns the token ID as a String if successful, or an error if the operation failed.
    // TODO: Replace the token ID type with a stronger typed token ID type (typically u256)
    fn promote_pending_nft(&mut self, tx_hash: String, token_id: String) -> eyre::Result<String>;

    /// Retrieves an already minted NFT from the database by its token ID.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The token ID of the NFT.
    ///
    /// # Returns
    ///
    /// Returns the NFT struct if found, or an error if the NFT doesn't exist in the minted NFT
    /// collection.
    // TODO: Replace the token ID type with a stronger typed token ID type (typically u256)
    fn get_nft(&self, token_id: String) -> eyre::Result<Nft>;

    /// Associates a tweet with an NFT token ID.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The token ID of the NFT.
    /// * `tweet_id` - The ID of the associated tweet.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the tweet was successfully associated, or an error if the operation
    /// failed.
    // TODO: Replace the token ID type with a stronger typed token ID type (typically u256)
    fn add_tweet(&mut self, token_id: String, tweet_id: String) -> eyre::Result<()>;

    /// Retrieves the tweet ID associated with an NFT token ID.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The token ID of the NFT.
    ///
    /// # Returns
    ///
    /// Returns the tweet ID as a String if found, or an error if no tweet is associated with the
    /// token ID.
    fn get_tweet(&self, token_id: String) -> eyre::Result<String>;

    /// Adds a new session to the database.
    ///
    /// # Arguments
    ///
    /// * `session` - The Session struct containing session information.
    ///
    /// # Returns
    ///
    /// Returns the session ID as a String if successfully added, or an error if the operation
    /// failed.
    fn add_session(&mut self, session: Session) -> eyre::Result<String>;

    /// Retrieves a session from the database by its ID.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The ID of the session.
    ///
    /// # Returns
    ///
    /// Returns the Session struct if found, or an error if the session doesn't exist.
    fn get_session(&self, session_id: String) -> eyre::Result<Session>;

    /// Serializes the database into a byte vector.
    ///
    /// # Returns
    ///
    /// Returns a vector of bytes representing the serialized database, or an error if serialization
    /// failed.
    fn serialize(&self) -> eyre::Result<Vec<u8>>;
}
