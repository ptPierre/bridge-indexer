use web3::{
    types::{Address, FilterBuilder, BlockNumber, U64},
    Web3,
};
use web3::transports::{WebSocket, Http};
use futures::StreamExt;
use eyre::Result;
use std::env;
use dotenv::dotenv;
use web3::ethabi::RawLog;
use crate::utils::abi::load_abi;
use std::path::Path;
use std::time::Duration;
use sqlx::postgres::PgPool;

use crate::models::transfers::Transfer;
use crate::repositories::transfers as transfers_repo;
use crate::utils::config::contracts;

pub struct IndexerConfig {
    pub days_to_backfill: Option<u64>,
    pub start_block: Option<u64>,
    pub batch_size: u64,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            days_to_backfill: None,
            start_block: None,
            batch_size: 100,
        }
    }
}

pub async fn start_indexer(config: IndexerConfig) -> Result<()> {
    dotenv().ok();
    
    // Initialize database connection
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    let pool = PgPool::connect(&database_url).await?;
    println!("Connected to PostgreSQL database");
    
    // USDC contract address
    let contract_address = contracts::usdc_address();
    println!("Monitoring USDC contract: {:?}", contract_address);
    
    // Load ABI
    let abi_path = Path::new("src/abis/erc20.json");
    let contract_abi = load_abi(abi_path)?;
    
    // Get Transfer event signature from ABI
    let transfer_event = contract_abi.event("Transfer")?;
    let event_signature = transfer_event.signature();
    
    // Check if historical backfill
    let should_backfill = config.days_to_backfill.is_some() || config.start_block.is_some();
    
    if should_backfill {
        // Use HTTP provider for historical data
        let http_url = env::var("HTTP_RPC_URL")
            .expect("HTTP_RPC_URL must be set in .env file");
        
        let transport = Http::new(&http_url)?;
        let web3 = Web3::new(transport);
        println!("Connected to HTTP provider for historical data");
        
        // Get current block
        let latest_block = web3.eth().block_number().await?;
        let latest_block_num = latest_block.as_u64();
        println!("Current block: {}", latest_block_num);
        
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
        
        println!("Backfilling blocks {} to {}", start_block, latest_block_num);
        
        // Process in batches
        let batch_size = config.batch_size;
        let mut current_batch_start = start_block;
        let mut total_transfers = 0;
        let mut transfer_buffer = Vec::with_capacity(1000);
        
        while current_batch_start <= latest_block_num {
            let batch_end = std::cmp::min(current_batch_start + batch_size - 1, latest_block_num);
            
            println!("Processing blocks {} to {}", current_batch_start, batch_end);
            
            // Create filter for the current batch of blocks
            let filter = FilterBuilder::default()
                .address(vec![contract_address])
                .topics(Some(vec![event_signature]), None, None, None)
                .from_block(BlockNumber::Number(U64::from(current_batch_start)))
                .to_block(BlockNumber::Number(U64::from(batch_end)))
                .build();
            
            // Get logs for the block range
            let logs = web3.eth().logs(filter).await?;
            println!("Found {} logs in block range", logs.len());
            
        
            for log in logs {
                // Extract block number and transaction hash
                let block_number = log.block_number.map(|bn| bn.as_u64());
                let tx_hash = log.transaction_hash.map(|h| format!("{:?}", h));
                
            
                let raw_log = RawLog {
                    topics: log.topics,
                    data: log.data.0,
                };
                
                // Decoding the Transfer event here
                match transfer_event.parse_log(raw_log) {
                    Ok(decoded_log) => {
                        // Extract params
                        let from = decoded_log.params[0].value.clone().into_address().unwrap();
                        let to = decoded_log.params[1].value.clone().into_address().unwrap();
                        let value = decoded_log.params[2].value.clone().into_uint().unwrap();
                        
                        // Create transfer record
                        match Transfer::new(from, to, value, block_number, tx_hash) {
                            Ok(transfer) => {
                                transfer_buffer.push(transfer);
                                
                                // Save in batches of 1000 transfers
                                if transfer_buffer.len() >= 1000 {
                                    match transfers_repo::save_batch(&pool, &transfer_buffer).await {
                                        Ok(()) => {
                                            total_transfers += transfer_buffer.len();
                                            println!("✅ Saved batch, total: {}", total_transfers);
                                        },
                                        Err(e) => eprintln!("❌ Failed to save batch: {:?}", e),
                                    }
                                    transfer_buffer.clear();
                                }
                            },
                            Err(e) => eprintln!("Error creating transfer record: {:?}", e),
                        }
                    },
                    Err(e) => eprintln!("❌ Error decoding event: {:?}", e),
                }
            }
            
            // Moving to next batch
            current_batch_start = batch_end + 1;
        }
        
        // Save any remaining transfers
        if !transfer_buffer.is_empty() {
            match transfers_repo::save_batch(&pool, &transfer_buffer).await {
                Ok(()) => {
                    total_transfers += transfer_buffer.len();
                    println!("✅ Saved final historical batch, total: {}", total_transfers);
                },
                Err(e) => eprintln!("❌ Failed to save final historical batch: {:?}", e),
            }
            transfer_buffer.clear();
        }
        
        println!("Backfill complete! Indexed {} transfers from blocks {} to {}", 
                 total_transfers, start_block, latest_block_num);
    }
    
    // Here the real-time indexing starts
    return start_realtime_indexing(pool, contract_address, &transfer_event).await;
}

// Extracted real-time indexing part to a separate function
async fn start_realtime_indexing(pool: sqlx::PgPool, contract_address: Address, transfer_event: &web3::ethabi::Event) -> Result<()> {
    println!("Starting real-time indexing...");
    
    // Get API key from env
    let api_key = env::var("WEBSOCKET_URL")
        .expect("WEBSOCKET_URL must be set in .env file");
    
    // Connect with WebSocket (for real-time indexing, better than periodic HTTP calls imo)
    let ws_url = format!("wss://eth-mainnet.g.alchemy.com/v2/{}", api_key);
    let transport = WebSocket::new(&ws_url).await?;
    let web3 = Web3::new(transport);
    println!("Connected to WebSocket provider");

    // Filter for Transfer events
    let event_signature = transfer_event.signature();
    let filter = FilterBuilder::default()
        .address(vec![contract_address])
        .topics(Some(vec![event_signature]), None, None, None)
        .build();

    // Create stream of logs
    let mut logs_stream = web3.eth_subscribe().subscribe_logs(filter).await?;
    println!("Subscribed to Transfer events in real-time");

    // transfer buffer
    let mut transfer_buffer = Vec::with_capacity(100);
    let batch_size = 100;

    // Processing logs as they arrive
    while let Some(log_result) = logs_stream.next().await {
        match log_result {
            Ok(log) => {
                println!("New Transfer event detected!");
                
                // Extract metadata from the log
                let block_number = log.block_number.map(|bn| bn.as_u64());
                let tx_hash = log.transaction_hash.map(|h| format!("{:?}", h));
                
                println!("  Block:       {:?}", block_number);
                println!("  Transaction: {:?}", tx_hash);
                
                let raw_log = RawLog {
                    topics: log.topics.clone(),
                    data: log.data.0.clone(),
                };
                
                // Decoding the Transfer event 
                match transfer_event.parse_log(raw_log) {
                    Ok(decoded_log) => {
                        // Extract parameters
                        let from = decoded_log.params[0].value.clone().into_address().unwrap();
                        let to = decoded_log.params[1].value.clone().into_address().unwrap();
                        let value = decoded_log.params[2].value.clone().into_uint().unwrap();
                        
                        println!("  From:        {:?}", from);
                        println!("  To:          {:?}", to);
                        println!("  Value:       {}", value);
                        
                        // Create transfer record and push to buffer
                        match Transfer::new(from, to, value, block_number, tx_hash) {
                            Ok(transfer) => {
                                transfer_buffer.push(transfer);
                                
                                // When buffer reaches batch size I save the batch
                                if transfer_buffer.len() >= batch_size {
                                    match transfers_repo::save_batch(&pool, &transfer_buffer).await {
                                        Ok(()) => println!("✅ Saved batch of {} transfers", transfer_buffer.len()),
                                        Err(e) => eprintln!("❌ Failed to save batch: {:?}", e),
                                    }
                                    // Clear the buffer after saving
                                    transfer_buffer.clear();
                                }
                            },
                            Err(e) => eprintln!("Error creating transfer record: {:?}", e),
                        }
                    },
                    Err(e) => eprintln!("❌ Error decoding event: {:?}", e),
                }
                println!();
            },
            Err(e) => eprintln!("Error in log stream: {:?}", e),
        }
    }
    
    // Save any remaining transfers
    tokio::time::sleep(Duration::from_secs(30)).await;
    if !transfer_buffer.is_empty() {
        match transfers_repo::save_batch(&pool, &transfer_buffer).await {
            Ok(()) => println!("✅ Saved periodic batch of {} transfers", transfer_buffer.len()),
            Err(e) => eprintln!("❌ Failed to save periodic batch: {:?}", e),
        }
    }

    Ok(())
} 