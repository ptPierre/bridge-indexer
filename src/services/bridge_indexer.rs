use web3::{
    types::{Address, FilterBuilder, BlockNumber, U64},
    Web3,
};
use web3::transports::{WebSocket, Http};
use futures::StreamExt;
use eyre::Result;
use std::env;
use web3::ethabi::RawLog;
use crate::utils::abi::load_abi;
use std::path::Path;
use std::time::Duration;
use sqlx::postgres::PgPool;


use crate::models::bridge::BridgeEvent;
use crate::repositories::bridge as bridge_repo;
use crate::utils::config::{contracts, networks};

#[derive(Clone)]
pub struct BridgeIndexerConfig {
    pub days_to_backfill: Option<u64>,
    pub start_block: Option<u64>,
    pub batch_size: u64,
}

impl Default for BridgeIndexerConfig {
    fn default() -> Self {
        Self {
            days_to_backfill: None,
            start_block: None,
            batch_size: 100,
        }
    }
}

pub async fn start_bridge_indexer(config: BridgeIndexerConfig) -> Result<()> {
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
        let config_clone = config.clone();
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
                        match index_network(
                            &network_name,
                            bridge_address,
                            &deposit_event,
                            &distribution_event,
                            config_clone,
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

async fn index_network(
    network: &str,
    contract_address: Address,
    deposit_event: &web3::ethabi::Event,
    distribution_event: &web3::ethabi::Event,
    config: BridgeIndexerConfig,
    pool: PgPool
) -> Result<()> {
    println!("Monitoring {} bridge contract: {:?}", network, contract_address);
    
    // Get network-specific RPC URL
    let http_url = networks::get_rpc_url(network);
    
    // Check if historical backfill
    let should_backfill = config.days_to_backfill.is_some() || config.start_block.is_some();
    
    if should_backfill {
        // Use HTTP provider for historical data
        let transport = Http::new(&http_url)?;
        let web3 = Web3::new(transport);
        println!("Connected to HTTP provider for {} historical data", network);
        
        // Get current block
        let latest_block = web3.eth().block_number().await?;
        let latest_block_num = latest_block.as_u64();
        println!("{} current block: {}", network, latest_block_num);
        
        // Calculate start block
        let start_block = if let Some(block) = config.start_block {
            block
        } else if let Some(days) = config.days_to_backfill {
            // Calculate block from days (assuming ~13.5 seconds per block)
            let blocks_per_day = 24 * 60 * 60 / 13;
            let blocks_to_go_back = days * blocks_per_day;
            latest_block_num.saturating_sub(blocks_to_go_back)
        } else {
            latest_block_num
        };
        
        println!("Backfilling {} blocks {} to {}", network, start_block, latest_block_num);
        
        // Process in batches
        let batch_size = config.batch_size;
        let mut current_batch_start = start_block;
        let mut total_events = 0;
        let mut event_buffer = Vec::with_capacity(1000);
        
        let deposit_signature = deposit_event.signature();
        let distribution_signature = distribution_event.signature();
        
        while current_batch_start <= latest_block_num {
            let batch_end = std::cmp::min(current_batch_start + batch_size - 1, latest_block_num);
            
            println!("Processing {} blocks {} to {}", network, current_batch_start, batch_end);
            
            // Create combined filter for both events
            let filter = FilterBuilder::default()
                .address(vec![contract_address])
                .topics(
                    Some(vec![deposit_signature, distribution_signature]), 
                    None, None, None
                )
                .from_block(BlockNumber::Number(U64::from(current_batch_start)))
                .to_block(BlockNumber::Number(U64::from(batch_end)))
                .build();
            
            // Get logs for the block range
            let logs = web3.eth().logs(filter).await?;
            println!("Found {} logs in {} block range", logs.len(), network);
            
            for log in logs {
                // Extract block number and transaction hash
                let block_number = log.block_number.map(|bn| bn.as_u64());
                let tx_hash = log.transaction_hash.map(|h| format!("{:?}", h));
                
                let raw_log = RawLog {
                    topics: log.topics.clone(),
                    data: log.data.0.clone(),
                };
                
                // Check which event it is
                if log.topics[0] == deposit_signature {
                    // Decoding the Deposit event
                    match deposit_event.parse_log(raw_log) {
                        Ok(decoded_log) => {
                            // Extract params
                            let token = decoded_log.params[0].value.clone().into_address().unwrap();
                            let from = decoded_log.params[1].value.clone().into_address().unwrap();
                            let to = decoded_log.params[2].value.clone().into_address().unwrap();
                            let amount = decoded_log.params[3].value.clone().into_uint().unwrap();
                            let nonce = decoded_log.params[4].value.clone().into_uint().unwrap();
                            
                            // Create bridge event record
                            match BridgeEvent::new_deposit(
                                network, token, from, to, amount, nonce, block_number, tx_hash
                            ) {
                                Ok(event) => {
                                    event_buffer.push(event);
                                }
                                Err(e) => eprintln!("Error creating deposit event record: {:?}", e),
                            }
                        },
                        Err(e) => eprintln!("❌ Error decoding deposit event: {:?}", e),
                    }
                } else if log.topics[0] == distribution_signature {
                    // Decoding the Distribution event
                    match distribution_event.parse_log(raw_log) {
                        Ok(decoded_log) => {
                            // Extract params
                            let token = decoded_log.params[0].value.clone().into_address().unwrap();
                            let to = decoded_log.params[1].value.clone().into_address().unwrap();
                            let amount = decoded_log.params[2].value.clone().into_uint().unwrap();
                            let nonce = decoded_log.params[3].value.clone().into_uint().unwrap();
                            
                            // Create bridge event record
                            match BridgeEvent::new_distribution(
                                network, token, to, amount, nonce, block_number, tx_hash
                            ) {
                                Ok(event) => {
                                    event_buffer.push(event);
                                }
                                Err(e) => eprintln!("Error creating distribution event record: {:?}", e),
                            }
                        },
                        Err(e) => eprintln!("❌ Error decoding distribution event: {:?}", e),
                    }
                }
                
                // Save in batches of 1000 events
                if event_buffer.len() >= 1000 {
                    match bridge_repo::save_batch(&pool, &event_buffer).await {
                        Ok(()) => {
                            total_events += event_buffer.len();
                            println!("✅ Saved {} batch, total: {}", network, total_events);
                        },
                        Err(e) => eprintln!("❌ Failed to save {} batch: {:?}", network, e),
                    }
                    event_buffer.clear();
                }
            }
            
            // Moving to next batch
            current_batch_start = batch_end + 1;
        }
        
        // Save any remaining events
        if !event_buffer.is_empty() {
            match bridge_repo::save_batch(&pool, &event_buffer).await {
                Ok(()) => {
                    total_events += event_buffer.len();
                    println!("✅ Saved final {} historical batch, total: {}", network, total_events);
                },
                Err(e) => eprintln!("❌ Failed to save final {} historical batch: {:?}", network, e),
            }
            event_buffer.clear();
        }
        
        println!("{} backfill complete! Indexed {} events from blocks {} to {}", 
                 network, total_events, start_block, latest_block_num);
    }
    
    // Real-time indexing
    start_realtime_bridge_indexing(network, pool, contract_address, deposit_event, distribution_event).await
}

async fn start_realtime_bridge_indexing(
    network: &str,
    pool: sqlx::PgPool,
    contract_address: Address,
    deposit_event: &web3::ethabi::Event,
    distribution_event: &web3::ethabi::Event
) -> Result<()> {
    println!("Starting real-time {} bridge indexing...", network);
    
    // Get network-specific WebSocket URL
    let ws_url = networks::get_rpc_url(&format!("{}_WS", network));
    
    // Connect with WebSocket
    let transport = WebSocket::new(&ws_url).await?;
    let web3 = Web3::new(transport);
    println!("Connected to WebSocket provider for {}", network);

    // Get event signatures
    let deposit_signature = deposit_event.signature();
    let distribution_signature = distribution_event.signature();

    // Filter for both events
    let filter = FilterBuilder::default()
        .address(vec![contract_address])
        .topics(
            Some(vec![deposit_signature, distribution_signature]), 
            None, None, None
        )
        .build();

    // Create stream of logs
    let mut logs_stream = web3.eth_subscribe().subscribe_logs(filter).await?;
    println!("Subscribed to bridge events on {} in real-time", network);

    // Event buffer
    let mut event_buffer = Vec::with_capacity(100);
    let batch_size = 100;

    // Processing logs as they arrive
    while let Some(log_result) = logs_stream.next().await {
        match log_result {
            Ok(log) => {
                println!("New bridge event detected on {}!", network);
                
                // Extract metadata from the log
                let block_number = log.block_number.map(|bn| bn.as_u64());
                let tx_hash = log.transaction_hash.map(|h| format!("{:?}", h));
                
                println!("  Block:       {:?}", block_number);
                println!("  Transaction: {:?}", tx_hash);
                
                let raw_log = RawLog {
                    topics: log.topics.clone(),
                    data: log.data.0.clone(),
                };
                
                // Check which event it is
                if log.topics[0] == deposit_signature {
                    // Decoding the Deposit event
                    match deposit_event.parse_log(raw_log) {
                        Ok(decoded_log) => {
                            // Extract params
                            let token = decoded_log.params[0].value.clone().into_address().unwrap();
                            let from = decoded_log.params[1].value.clone().into_address().unwrap();
                            let to = decoded_log.params[2].value.clone().into_address().unwrap();
                            let amount = decoded_log.params[3].value.clone().into_uint().unwrap();
                            let nonce = decoded_log.params[4].value.clone().into_uint().unwrap();
                            
                            println!("  Event:       Deposit");
                            println!("  Token:       {:?}", token);
                            println!("  From:        {:?}", from);
                            println!("  To:          {:?}", to);
                            println!("  Amount:      {}", amount);
                            println!("  Nonce:       {}", nonce);
                            
                            // Create bridge event record
                            match BridgeEvent::new_deposit(
                                network, token, from, to, amount, nonce, block_number, tx_hash
                            ) {
                                Ok(event) => {
                                    event_buffer.push(event);
                                }
                                Err(e) => eprintln!("Error creating deposit event record: {:?}", e),
                            }
                        },
                        Err(e) => eprintln!("❌ Error decoding deposit event: {:?}", e),
                    }
                } else if log.topics[0] == distribution_signature {
                    // Decoding the Distribution event
                    match distribution_event.parse_log(raw_log) {
                        Ok(decoded_log) => {
                            // Extract params
                            let token = decoded_log.params[0].value.clone().into_address().unwrap();
                            let to = decoded_log.params[1].value.clone().into_address().unwrap();
                            let amount = decoded_log.params[2].value.clone().into_uint().unwrap();
                            let nonce = decoded_log.params[3].value.clone().into_uint().unwrap();
                            
                            println!("  Event:       Distribution");
                            println!("  Token:       {:?}", token);
                            println!("  To:          {:?}", to);
                            println!("  Amount:      {}", amount);
                            println!("  Nonce:       {}", nonce);
                            
                            // Create bridge event record
                            match BridgeEvent::new_distribution(
                                network, token, to, amount, nonce, block_number, tx_hash
                            ) {
                                Ok(event) => {
                                    event_buffer.push(event);
                                }
                                Err(e) => eprintln!("Error creating distribution event record: {:?}", e),
                            }
                        },
                        Err(e) => eprintln!("❌ Error decoding distribution event: {:?}", e),
                    }
                }
                
                // When buffer reaches batch size I save the batch
                if event_buffer.len() >= batch_size {
                    match bridge_repo::save_batch(&pool, &event_buffer).await {
                        Ok(()) => println!("✅ Saved batch of {} {} bridge events", event_buffer.len(), network),
                        Err(e) => eprintln!("❌ Failed to save batch: {:?}", e),
                    }
                    // Clear the buffer after saving
                    event_buffer.clear();
                }
                
                println!();
            },
            Err(e) => eprintln!("Error in {} log stream: {:?}", network, e),
        }
    }
    
    // Save any remaining events periodically
    tokio::time::sleep(Duration::from_secs(30)).await;
    if !event_buffer.is_empty() {
        match bridge_repo::save_batch(&pool, &event_buffer).await {
            Ok(()) => println!("✅ Saved periodic batch of {} {} bridge events", event_buffer.len(), network),
            Err(e) => eprintln!("❌ Failed to save periodic batch: {:?}", e),
        }
    }

    Ok(())
} 