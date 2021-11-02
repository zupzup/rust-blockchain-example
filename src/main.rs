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
    pub data: String,
}

impl Block {
    pub fn new(id: u64, previous_hash: String, data: String) -> Self {
        let now = Utc::now();
        let hash = calculate_hash(id, now.timestamp(), &previous_hash, &data);
        Self {
            id,
            hash,
            timestamp: now.timestamp(),
            previous_hash,
            data,
        }
    }
}

fn calculate_hash(id: u64, timestamp: i64, previous_hash: &str, data: &str) -> String {
    let data = serde_json::json!({
        "id": id,
        "previous_hash": previous_hash,
        "data": data,
        "timestamp": timestamp
    });
    // println!("block data: {}", data.to_string());
    hex_digest(Algorithm::SHA256, data.to_string().as_bytes())
}

struct App {
    pub node_id: String,
    pub nodes: Vec<String>,
    pub blocks: Vec<Block>,
}

impl App {
    fn genesis() -> Self {
        let genesis_block = Block::new(0, String::from("genesis"), String::from("genesis!"));
        Self {
            node_id: Uuid::new_v4().to_string(),
            nodes: vec![],
            blocks: vec![genesis_block],
        }
    }

    fn generate_new_block(&mut self) {
        let latest_block = self.blocks.last().expect("there is at least one block");

        let block = Block::new(
            latest_block.id + 1,
            latest_block.hash.clone(),
            String::from("new block data!"),
        );
        self.blocks.push(block);
    }

    fn is_block_valid(&mut self, block: &Block) -> bool {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if block.hash != latest_block.hash {
            return false;
        } else if block.id != latest_block.id + 1 {
            return false;
        } else if calculate_hash(block.id, block.timestamp, &block.previous_hash, &block.data)
            != block.hash
        {
            return false;
        }
        true
    }

    fn is_chain_valid(&mut self, chain: &Vec<Block>) -> bool {
        // TODO
        true
    }

    fn choose_longer_chain(&mut self, local: &Vec<Block>, remote: &Vec<Block>) {
        // TODO: choose the longer chain
        // validate both chains
        // choose chain with bigger height (i.e. where len() is bigger)
    }
}

fn main() {
    let mut app = App::genesis();
    println!("Started a node with id: {}", app.node_id);

    app.generate_new_block();
    app.generate_new_block();
    app.generate_new_block();

    let is_block_valid = app.is_block_valid(&Block::new(
        12,
        String::from("yay"),
        String::from("yay block"),
    ));
    println!("block valid: {}", is_block_valid);

    let serialized_chain = serde_json::to_string_pretty(&app.blocks).expect("serialize blocks");

    println!("Blocks: {}", serialized_chain);
    println!("connected nodes: {:?}", app.nodes);
}
