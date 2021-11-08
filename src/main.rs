use chrono::prelude::*;
use crypto_hash::{digest, Algorithm};
use libp2p::{
    core::upgrade,
    floodsub::Floodsub,
    futures::StreamExt,
    mdns::Mdns,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    Transport,
};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncBufReadExt, sync::mpsc};
use uuid::Uuid;

const DIFFICULTY_PREFIX: &str = "00";

mod p2p;

#[derive(Serialize, Deserialize, Debug)]
struct Block {
    pub id: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: i64,
    pub data: String,
    pub nonce: u64,
}

impl Block {
    pub fn new(id: u64, previous_hash: String, data: String) -> Self {
        let now = Utc::now();
        let (nonce, hash) = mine_block(id, now.timestamp(), &previous_hash, &data);
        Self {
            id,
            hash,
            timestamp: now.timestamp(),
            previous_hash,
            data,
            nonce,
        }
    }
}

fn calculate_hash(id: u64, timestamp: i64, previous_hash: &str, data: &str, nonce: u64) -> Vec<u8> {
    let data = serde_json::json!({
        "id": id,
        "previous_hash": previous_hash,
        "data": data,
        "timestamp": timestamp,
        "nonce": nonce
    });
    // println!("block data: {}", data.to_string());
    digest(Algorithm::SHA256, data.to_string().as_bytes())
}

fn mine_block(id: u64, timestamp: i64, previous_hash: &str, data: &str) -> (u64, String) {
    println!("mining block...");
    let mut nonce = 0;

    loop {
        if nonce % 100000 == 0 {
            println!("nonce: {}", nonce);
        }
        let hash = calculate_hash(id, timestamp, previous_hash, data, nonce);
        let binary_hash = hash_to_binary_representation(&hash);
        if binary_hash.starts_with(DIFFICULTY_PREFIX) {
            println!(
                "mined! nonce: {}, hash: {}, binary hash: {}",
                nonce,
                hex::encode(&hash),
                binary_hash
            );
            return (nonce, hex::encode(hash));
        }
        nonce += 1;
    }
}

fn hash_to_binary_representation(hash: &Vec<u8>) -> String {
    let mut res: String = String::default();
    for c in hash {
        res.push_str(&format!("{:b}", c));
    }
    res
}

struct App {
    pub node_id: String,
    pub nodes: Vec<String>,
    pub blocks: Vec<Block>,
}

impl App {
    fn genesis() -> Self {
        let genesis_block = Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("genesis"),
            data: String::from("genesis!"),
            nonce: 2836,
            hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
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

    fn is_block_valid(&self, block: &Block) -> bool {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if block.previous_hash != latest_block.hash {
            return false;
        } else if !hash_to_binary_representation(
            &hex::decode(&block.hash).expect("can decode from hex"),
        )
        .starts_with(DIFFICULTY_PREFIX)
        {
            return false;
        } else if block.id != latest_block.id + 1 {
            return false;
        } else if hex::encode(calculate_hash(
            block.id,
            block.timestamp,
            &block.previous_hash,
            &block.data,
            block.nonce,
        )) != block.hash
        {
            return false;
        }
        true
    }

    fn is_chain_valid(&self, chain: &Vec<Block>) -> bool {
        chain.iter().all(|b| self.is_block_valid(b))
    }

    // We always choose the longest valid chain
    fn choose_chain(&mut self, local: Vec<Block>, remote: Vec<Block>) -> Vec<Block> {
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);

        if is_local_valid && is_remote_valid {
            return if local.len() >= remote.len() {
                local
            } else {
                remote
            };
        } else if is_remote_valid && !is_local_valid {
            return remote;
        } else if !is_remote_valid && is_local_valid {
            return local;
        } else {
            panic!("local and remote chains are both invalid");
        }
    }
}
#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let mut app = App::genesis();
    println!("Started a node with id: {}", app.node_id);

    app.generate_new_block();
    let latest_block = app.blocks.last().expect("there is a block");
    app.is_block_valid(&latest_block);

    // let is_block_valid = app.is_block_valid(&Block::new(
    //     12,
    //     String::from("yay"),
    //     String::from("yay block"),
    // ));
    // println!("block valid: {}", is_block_valid);

    let serialized_chain = serde_json::to_string_pretty(&app.blocks).expect("serialize blocks");

    println!("Blocks: {}", serialized_chain);
    println!("connected nodes: {:?}", app.nodes);

    // -------------------------------------------- p2p stuff
    info!("Peer Id: {}", p2p::PEER_ID.clone());
    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&p2p::KEYS)
        .expect("can create auth keys");

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated()) // XX Handshake pattern, IX exists as well and IK - only XX currently provides interop with other libp2p impls
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let mut behaviour = p2p::AppBehaviour {
        floodsub: Floodsub::new(p2p::PEER_ID.clone()),
        mdns: Mdns::new(Default::default())
            .await
            .expect("can create mdns"),
        response_sender,
    };

    behaviour.floodsub.subscribe(p2p::TOPIC.clone());

    let mut swarm = SwarmBuilder::new(transp, behaviour, p2p::PEER_ID.clone())
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
    .expect("swarm can be started");

    loop {
        let evt = {
            tokio::select! {
                line = stdin.next_line() => Some(p2p::EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                response = response_rcv.recv() => Some(p2p::EventType::Response(response.expect("response exists"))),
                event = swarm.select_next_some() => {
                    info!("Unhandled Swarm Event: {:?}", event);
                    None
                },
            }
        };

        if let Some(event) = evt {
            match event {
                p2p::EventType::Response(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(p2p::TOPIC.clone(), json.as_bytes());
                }
                p2p::EventType::Input(line) => match line.as_str() {
                    "ls p" => p2p::handle_list_peers(&mut swarm).await,
                    cmd if cmd.starts_with("ls r") => {
                        p2p::handle_list_recipes(cmd, &mut swarm).await
                    }
                    cmd if cmd.starts_with("create r") => p2p::handle_create_recipe(cmd).await,
                    cmd if cmd.starts_with("publish r") => p2p::handle_publish_recipe(cmd).await,
                    _ => error!("unknown command"),
                },
            }
        }
    }
}
