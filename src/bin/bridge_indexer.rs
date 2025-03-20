use dotenv::dotenv;
use lobster::services::bridge_indexer;
use clap::Parser;
use eyre::Result;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about = "Bridge Event Indexer")]
struct Args {
    /// Batch size for event processing
    #[clap(short, long, default_value = "100")]
    batch_size: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    
    // Parse command line args
    let args = Args::parse();
    
    // Create indexer config
    let config = bridge_indexer::BridgeIndexerConfig {
        batch_size: args.batch_size,
    };
    
    // Start bridge indexer
    bridge_indexer::start_bridge_indexer().await
} 