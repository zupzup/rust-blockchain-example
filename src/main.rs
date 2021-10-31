use chrono::prelude::*;
use crypto_hash::{hex_digest, Algorithm};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

type Creator = String;

#[derive(Serialize, Deserialize, Debug)]
struct Block {
    pub id: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: i64,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(id: u64, previous_hash: String, transactions: Vec<Transaction>) -> Self {
        let now = Utc::now();
        let hash = calculate_hash(id, now.timestamp(), &previous_hash, &transactions);
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
    id: u64,
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
    // println!("block data: {}", data.to_string());
    hex_digest(Algorithm::SHA256, data.to_string().as_bytes())
}

#[derive(Serialize, Deserialize, Debug)]
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
    pub blocks: Vec<Block>,
}

impl App {
    fn genesis() -> Self {
        let genesis_block = Block::new(0, String::from("genesis"), vec![]);
        Self {
            node_id: Uuid::new_v4().to_string(),
            nodes: vec![],
            current_transactions: vec![],
            blocks: vec![genesis_block],
        }
    }

    fn generate_new_block(&mut self) {
        let latest_block = self.blocks.last().expect("there is at least one block");

        let mut transactions = vec![];
        transactions.append(&mut self.current_transactions); // move current transactions over to this block

        let block = Block::new(latest_block.id + 1, latest_block.hash.clone(), transactions);
        self.blocks.push(block);
    }

    fn is_block_valid(&mut self, block: &Block) -> bool {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if block.hash != latest_block.hash {
            return false;
        } else if block.id != latest_block.id + 1 {
            return false;
        } else if calculate_hash(
            block.id,
            block.timestamp,
            &block.previous_hash,
            &block.transactions,
        ) != block.hash
        {
            return false;
        }
        true
    }
}

fn main() {
    let mut app = App::genesis();
    println!("Started a node with id: {}", app.node_id);

    app.current_transactions
        .push(Transaction::new(String::from("test1"), app.node_id.clone()));

    app.generate_new_block();
    app.current_transactions
        .push(Transaction::new(String::from("test2"), app.node_id.clone()));
    app.current_transactions
        .push(Transaction::new(String::from("test3"), app.node_id.clone()));
    app.generate_new_block();
    app.generate_new_block();

    let is_block_valid = app.is_block_valid(&Block::new(12, String::from("yay"), vec![]));
    println!("block valid: {}", is_block_valid);

    let serialized_chain = serde_json::to_string_pretty(&app.blocks).expect("serialize blocks");

    println!("Blocks: {}", serialized_chain);
    println!("connected nodes: {:?}", app.nodes);
    println!("current transactions: {:?}", app.current_transactions);
}
