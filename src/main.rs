use alloy::signers::local::{coins_bip39::English, MnemonicBuilder};
use db::{create_tables, open_connection};
use endpoints::{callback, mint, new_user, SharedState};
use listener::subscribe_to_events;
mod db;
mod endpoints;
mod listener;
mod oai;
mod twitter;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    let db_url = std::env::var("DB_URL").expect("DB_URL not set");
    let ws_rpc_url = std::env::var("WS_RPC_URL").expect("WS_RPC_URL not set");
    let mnemonic = std::env::var("NFT_MINTER_MNEMONIC").expect("NFT_MINTER_MNEMONIC not set");

    let signer = MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .index(0)
        .unwrap()
        .build()
        .unwrap();

    let mut connection = open_connection(db_url.clone()).expect("Failed to open database");
    create_tables(&mut connection).expect("Failed to create tables");

    let shared_state = SharedState {
        db_url: db_url.clone(),
        ws_rpc_url: ws_rpc_url.clone(),
        wallet: signer.into(),
    };
    let app = axum::Router::new()
        .route("/new", axum::routing::get(new_user))
        .route("/callback", axum::routing::get(callback))
        .route("/mint", axum::routing::get(mint))
        .with_state(shared_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    subscribe_to_events(&mut connection, ws_rpc_url)
        .await
        .unwrap();
}
