use web3::{
    types::{Address, FilterBuilder, TransactionParameters, U256, Bytes},
    Web3,
};
use web3::transports::{WebSocket, Http};
use futures::StreamExt;
use eyre::Result;
use std::env;
use web3::ethabi::{RawLog, Token, Function, Param, ParamType};
use crate::utils::abi::load_abi;
use std::path::Path;
use std::time::Duration;
use sqlx::postgres::PgPool;
use std::str::FromStr;
use secp256k1::{PublicKey, SecretKey, Secp256k1, Message};
use rlp::RlpStream;

use crate::models::bridge::BridgeEvent;
use crate::repositories::bridge as bridge_repo;
use crate::utils::config::{contracts, networks};

// Token addresses for cross-chain distribution
const SEPOLIA_TOKEN_ADDRESS: &str = "0x4D77a078a8f698b73b449866ec620DbDc921df39";
const HOLESKY_TOKEN_ADDRESS: &str = "0xFdA8C8E54219577c73C49441E5d86b512ACEfC28";

// At the top, add these constants
const SEPOLIA_CHAIN_ID: u64 = 11155111;
const HOLESKY_CHAIN_ID: u64 = 17000;

#[derive(Clone)]
pub struct BridgeIndexerConfig {
    pub batch_size: u64,
}

impl Default for BridgeIndexerConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
        }
    }
}

// Convert public key to Ethereum address
fn public_key_to_address(pubkey: &PublicKey) -> Address {
    let pubkey_bytes = pubkey.serialize_uncompressed();
    let hash = web3::signing::keccak256(&pubkey_bytes[1..]); // Skip the 0x04 prefix
    Address::from_slice(&hash[12..]) // Last 20 bytes
}

// Encode unsigned transaction for signing (EIP-155)
fn encode_unsigned_transaction(tx: &TransactionParameters, chain_id: u64) -> Vec<u8> {
    let mut stream = RlpStream::new_list(9);
    stream.append(&tx.nonce.unwrap_or_default());
    stream.append(&tx.gas_price.unwrap_or_default());
    stream.append(&tx.gas);
    stream.append(&tx.to.unwrap_or_default());
    stream.append(&tx.value);
    stream.append(&tx.data.0);
    stream.append(&chain_id);
    stream.append(&0u8);
    stream.append(&0u8);
    stream.out().to_vec()
}

// Encode signed transaction
fn encode_signed_transaction(tx: &TransactionParameters, v: u64, r: &[u8], s: &[u8]) -> Vec<u8> {
    let mut stream = RlpStream::new_list(9);
    stream.append(&tx.nonce.unwrap_or_default());
    stream.append(&tx.gas_price.unwrap_or_default());
    stream.append(&tx.gas);
    stream.append(&tx.to.unwrap_or_default());
    stream.append(&tx.value);
    stream.append(&tx.data.0);
    stream.append(&v);
    stream.append(&r);
    stream.append(&s);
    stream.out().to_vec()
}

pub async fn start_bridge_indexer() -> Result<()> {
    // Initialize database connection
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    let pool = PgPool::connect(&database_url).await?;
    println!("Connected to PostgreSQL database");
    
    // Start indexers for different networks
    let networks = vec!["sepolia", "holesky"];
    
    for network in networks {
        println!("Starting {} indexer...", network);
        
        // Get network-specific address
        let bridge_address = match network {
            "sepolia" => contracts::sepolia_bridge_address(),
            "holesky" => contracts::holesky_bridge_address(),
            _ => panic!("Unsupported network: {}", network),
        };
        
        // Spawn a task for each network
        let pool_clone = pool.clone();
        let network_name = network.to_string();
        
        tokio::spawn(async move {
            // Load ABI inside each task to avoid borrowing issues
            let abi_path = Path::new("src/abis/bridge.json");
            match load_abi(abi_path) {
                Ok(bridge_abi) => {
                    // Create events inside the task
                    if let (Ok(deposit_event), Ok(distribution_event)) = (
                        bridge_abi.event("Deposit"), 
                        bridge_abi.event("Distribution")
                    ) {
                        match monitor_network_events(
                            &network_name,
                            bridge_address,
                            &deposit_event,
                            &distribution_event,
                            pool_clone
                        ).await {
                            Ok(_) => println!("{} indexer completed successfully", network_name),
                            Err(e) => eprintln!("{} indexer error: {:?}", network_name, e),
                        }
                    } else {
                        eprintln!("Failed to get events from ABI for {}", network_name);
                    }
                },
                Err(e) => eprintln!("Failed to load ABI for {}: {:?}", network_name, e),
            }
        });
    }
    
    // Keep the main task alive
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

async fn monitor_network_events(
    network: &str,
    contract_address: Address,
    deposit_event: &web3::ethabi::Event,
    distribution_event: &web3::ethabi::Event,
    pool: PgPool
) -> Result<()> {
    println!("🔍 Monitoring {} bridge contract: {:?}", network, contract_address);
    
    // Get network-specific WebSocket RPC URL for monitoring
    let ws_url = networks::get_rpc_url(&format!("{}_WS", network));
    
    // Also get HTTP RPC URLs for both networks (for sending transactions)
    let sepolia_http_url = networks::get_rpc_url("SEPOLIA");
    let holesky_http_url = networks::get_rpc_url("HOLESKY");
    
    // Get private key for transaction signing
    let private_key = env::var("PRIVATE_KEY")
        .expect("PRIVATE_KEY must be set in .env file");

    // Remove 0x prefix if present
    let private_key_hex = if private_key.starts_with("0x") {
        private_key[2..].to_string()
    } else {
        private_key
    };

    // Connect to WebSocket provider for live data
    let transport = WebSocket::new(&ws_url).await?;
    let web3 = Web3::new(transport);
    println!("🔌 Connected to WebSocket provider for {} live data", network);
    
    // Create filter for both events
    let filter = FilterBuilder::default()
        .address(vec![contract_address])
        .build();
    
    // Subscribe to logs
    let mut logs_stream = web3.eth_subscribe().subscribe_logs(filter).await?;
    println!("📡 Subscribed to {} logs", network);
    
    // Create HTTP connections for both networks (for sending transactions)
    let sepolia_http = Http::new(&sepolia_http_url)?;
    let sepolia_web3 = Web3::new(sepolia_http);
    
    let holesky_http = Http::new(&holesky_http_url)?;
    let holesky_web3 = Web3::new(holesky_http);
    
    // Create distribute function signature
    let distribute_function = Function {
        name: "distribute".into(),
        inputs: vec![
            Param { name: "token".into(), kind: ParamType::Address, internal_type: None },
            Param { name: "recipient".into(), kind: ParamType::Address, internal_type: None },
            Param { name: "amount".into(), kind: ParamType::Uint(256), internal_type: None },
            Param { name: "depositNonce".into(), kind: ParamType::Uint(256), internal_type: None },
        ],
        outputs: vec![],
        constant: false,
        state_mutability: web3::ethabi::StateMutability::NonPayable,
    };
    
    // Get signatures
    let deposit_signature = deposit_event.signature();
    let distribution_signature = distribution_event.signature();
    
    // Process logs as they arrive
    while let Some(log) = logs_stream.next().await {
        match log {
            Ok(log) => {
                println!("\n🔔 {} New event detected!", network);
                println!("  Block:       {:?}", log.block_number);
                println!("  Transaction: {:?}", log.transaction_hash);
                
                let block_number = log.block_number.map(|bn| bn.as_u64());
                let tx_hash = log.transaction_hash.map(|h| format!("{:?}", h));
                
                // Store the first topic before moving log.topics
                let first_topic = log.topics[0];
                
                // Convert to raw log for ethabi
                let raw_log = RawLog {
                    topics: log.topics,  // This moves log.topics
                    data: log.data.0,
                };
                
                // Now use first_topic for comparisons
                if first_topic == deposit_signature {
                    // Decoding the Deposit event
                    match deposit_event.parse_log(raw_log) {
                        Ok(decoded_log) => {
                            // Extract params
                            let token = decoded_log.params[0].value.clone().into_address().unwrap();
                            let from = decoded_log.params[1].value.clone().into_address().unwrap();
                            let to = decoded_log.params[2].value.clone().into_address().unwrap();
                            let amount = decoded_log.params[3].value.clone().into_uint().unwrap();
                            let nonce = decoded_log.params[4].value.clone().into_uint().unwrap();
                            
                            println!("  Event:       📥 Deposit");
                            println!("  Token:       {:?}", token);
                            println!("  From:        {:?}", from);
                            println!("  To:          {:?}", to);
                            println!("  Amount:      {}", amount);
                            println!("  Nonce:       {}", nonce);
                            
                            // Create bridge event record and save directly
                            match BridgeEvent::new_deposit(
                                network, token, from, to, amount, nonce, block_number, tx_hash.clone()
                            ) {
                                Ok(event) => {
                                    if let Err(e) = bridge_repo::save_bridge_event(&pool, &event).await {
                                        eprintln!("❌ Error saving deposit event: {:?}", e);
                                    } else {
                                        println!("✅ Saved deposit event to database");
                                    }
                                },
                                Err(e) => eprintln!("❌ Error creating deposit event record: {:?}", e),
                            }
                            
                            // Now create a distribution transaction on the other chain
                            let (target_network, target_web3, target_address, token_address, target_chain_id) = if network == "sepolia" {
                                ("holesky", &holesky_web3, contracts::holesky_bridge_address(), HOLESKY_TOKEN_ADDRESS, HOLESKY_CHAIN_ID)
                            } else {
                                ("sepolia", &sepolia_web3, contracts::sepolia_bridge_address(), SEPOLIA_TOKEN_ADDRESS, SEPOLIA_CHAIN_ID)
                            };
                            
                            println!("\n🔄 Creating distribution transaction on {} network", target_network);
                            println!("  Target bridge: {:?}", target_address);
                            println!("  Token:         {}", token_address);
                            println!("  Recipient:     {:?}", to);
                            println!("  Amount:        {}", amount);
                            println!("  Nonce:         {}", nonce);

                            // Create the function call data
                            let token_address = Address::from_str(token_address).expect("Invalid token address");
                            let call_data = distribute_function.encode_input(&[
                                Token::Address(token_address),
                                Token::Address(to),
                                Token::Uint(amount),
                                Token::Uint(nonce),
                            ])?;

                            println!("📝 Transaction created and ready to send");

                            // Parse the private key and derive the from address
                            let secp = Secp256k1::new();
                            let secret_key = match SecretKey::from_str(&private_key_hex) {
                                Ok(key) => key,
                                Err(e) => {
                                    eprintln!("⚠️ Error parsing private key: {:?}", e);
                                    return Err(e.into()); // Handle error appropriately
                                }
                            };
                            let public_key = PublicKey::from_secret_key(&secp, &secret_key);
                            let from_address = public_key_to_address(&public_key);

                            // Create transaction parameters
                            let mut tx_request = TransactionParameters {
                                to: Some(target_address),
                                data: call_data.clone().into(),
                                gas: U256::from(300000),
                                chain_id: Some(target_chain_id),
                                ..Default::default()
                            };

                            // Get gas price and nonce from the network
                            if let Ok(gas_price) = target_web3.eth().gas_price().await {
                                tx_request.gas_price = Some(gas_price);
                            }
                            if let Ok(nonce) = target_web3.eth().transaction_count(from_address, None).await {
                                tx_request.nonce = Some(nonce);
                            }

                            // Sign the transaction manually
                            let unsigned_rlp = encode_unsigned_transaction(&tx_request, target_chain_id);
                            let hash = web3::signing::keccak256(&unsigned_rlp);
                            let message = Message::from_slice(&hash)?;
                            let signature = secp.sign(&message, &secret_key);
                            let sig_bytes = signature.serialize_compact();
                            let rec_id = 0; 
                            let r = &sig_bytes[0..32];
                            let s = &sig_bytes[32..64];
                            let v = target_chain_id * 2 + 35 + rec_id as u64; 
                            let raw_tx = encode_signed_transaction(&tx_request, v, r, s);

                            // Send the raw transaction
                            match target_web3.eth().send_raw_transaction(raw_tx.into()).await {
                                Ok(tx_hash) => {
                                    println!("🚀 Distribution transaction sent: {:?}", tx_hash);
                                }
                                Err(e) => {
                                    eprintln!("⚠️ Error sending transaction: {:?}", e);
                                    println!("⚠️ Falling back to simulation mode");
                                    println!("🔄 Would distribute: {} tokens to {:?} with nonce {}", amount, to, nonce);
                                }
                            }
                        },
                        Err(e) => eprintln!("❌ Error decoding deposit event: {:?}", e),
                    }
                } else if first_topic == distribution_signature {
                    // Decoding the Distribution event
                    match distribution_event.parse_log(raw_log) {
                        Ok(decoded_log) => {
                            // Extract params
                            let token = decoded_log.params[0].value.clone().into_address().unwrap();
                            let to = decoded_log.params[1].value.clone().into_address().unwrap();
                            let amount = decoded_log.params[2].value.clone().into_uint().unwrap();
                            let nonce = decoded_log.params[3].value.clone().into_uint().unwrap();
                            
                            println!("  Event:       📤 Distribution");
                            println!("  Token:       {:?}", token);
                            println!("  To:          {:?}", to);
                            println!("  Amount:      {}", amount);
                            println!("  Nonce:       {}", nonce);
                            
                            // Create bridge event record and save directly
                            match BridgeEvent::new_distribution(
                                network, token, to, amount, nonce, block_number, tx_hash
                            ) {
                                Ok(event) => {
                                    if let Err(e) = bridge_repo::save_bridge_event(&pool, &event).await {
                                        eprintln!("❌ Error saving distribution event: {:?}", e);
                                    } else {
                                        println!("✅ Saved distribution event to database");
                                    }
                                },
                                Err(e) => eprintln!("❌ Error creating distribution event record: {:?}", e),
                            }
                        },
                        Err(e) => eprintln!("❌ Error decoding distribution event: {:?}", e),
                    }
                }
                
                println!();
            },
            Err(e) => eprintln!("❌ Error in {} log stream: {:?}", network, e),
        }
    }
    
    println!("📢 {} event stream ended", network);
    Ok(())
} 