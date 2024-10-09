use std::{path::Path, sync::Arc};

use tokio::sync::Mutex;

use super::{in_memory::InMemoryDB, TeleportDB};

pub async fn load_or_create_db(db_path: &str) -> InMemoryDB {
    let path = Path::new(db_path);
    if path.exists() {
        let serialized_bytes = tokio::fs::read(&path).await.expect("Failed to read db file");
        let db = InMemoryDB::deserialize(&serialized_bytes);
        log::info!("Loaded db from file: {}", db_path);
        db
    } else {
        InMemoryDB::new()
    }
}

pub async fn save_db_on_shutdown(db: Arc<Mutex<InMemoryDB>>, db_path: &str) {
    let db = db.lock().await;
    let serialized = db.serialize().unwrap();
    let serialized_bytes = serialized.to_vec();
    tokio::fs::write(db_path, serialized_bytes)
        .await
        .expect("Failed to save serialized data to file");
    log::info!("Saved db to file: {}", db_path);
}
