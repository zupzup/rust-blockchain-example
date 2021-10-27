use chrono::prelude::*;
use crypto_hash::{digest, Algorithm};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

type Creator = String;

struct Block {
    pub id: String,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: i64,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(previous_hash: String, transactions: Vec<Transaction>) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let hash = calculate_hash(&id, now.timestamp(), &previous_hash, &transactions);
        Self {
            id,
            hash,
            timestamp: now.timestamp(),
            previous_hash,
            transactions,
        }
    }
}

fn calculate_hash(
    id: &str,
    timestamp: i64,
    previous_hash: &str,
    transactions: &Vec<Transaction>,
) -> String {
    let data = serde_json::json!({
        "id": id,
        "previous_hash": previous_hash,
        "transactions": transactions,
        "timestamp": timestamp
    });
    // TODO: serialize all data to a string (json)
    std::str::from_utf8(&digest(Algorithm::SHA256, data.to_string().as_bytes()))
        .expect("can create string from sha256 hash")
        .to_owned()
}

#[derive(Serialize, Deserialize)]
struct Transaction {
    pub id: String,
    pub data: String,
    pub timestamp: i64,
    pub creator: Creator,
}

impl Transaction {
    pub fn new(data: String, creator: Creator) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            data,
            timestamp: Utc::now().timestamp(),
            creator,
        }
    }
}

struct App {
    pub node_id: String,
    pub nodes: Vec<String>,
    pub current_transactions: Vec<Transaction>,
}

fn main() {
    println!("Hello, world!");
}
