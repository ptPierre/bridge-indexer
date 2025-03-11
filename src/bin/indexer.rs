use dotenv::dotenv;
use lobster::services::indexer::{start_indexer, IndexerConfig};
use clap::Parser;
use eyre::Result;

#[derive(Parser, Debug)]
#[clap(author, version, about = "USDC Transfer Indexer")]
struct Args {
    /// Days to go back in history (default: 0, meaning real-time only)
    #[clap(short, long)]
    days: Option<u64>,
    
    /// Start block (overrides days)
    #[clap(short, long)]
    start_block: Option<u64>,
    
    /// Batch size for processing historical blocks
    #[clap(short, long, default_value = "100")]
    batch_size: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    
    // Parse command line args
    let args = Args::parse();
    
    // Create indexer config
    let config = IndexerConfig {
        days_to_backfill: args.days,
        start_block: args.start_block,
        batch_size: args.batch_size,
    };
    
    // Start indexer
    start_indexer(config).await
} 