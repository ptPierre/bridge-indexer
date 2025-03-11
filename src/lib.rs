use rocket::{get, routes};

pub mod api;
pub mod models;
pub mod services;
pub mod repositories;
pub mod utils;
use clap::Parser;
use eyre::Result;
use rocket::{Build, Rocket};
use api::transfers::get_transfers;
use tokio::task;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Lobster - USDC Transfer Indexer + API")]
pub struct AppArgs {
    /// Days to go back in history (default 0, real-time only)
    #[clap(short, long)]
    pub days: Option<u64>,
    
    /// Start block (overrides days)
    #[clap(short, long)]
    pub start_block: Option<u64>,
    
    /// Batch size 
    #[clap(short, long, default_value = "100")]
    pub batch_size: u64,
    
    /// Skip starting the indexer
    #[clap(long)]
    pub api_only: bool,
}

/// Initializes the application with the given arguments
pub async fn start_app(args: AppArgs) -> Result<Rocket<Build>> {
    use std::env;
    use rocket::response::content::RawHtml;
    use services::indexer::{start_indexer, IndexerConfig};

    println!("Starting application...");
    
    // Create indexer config from command line args
    let config = IndexerConfig {
        days_to_backfill: args.days,
        start_block: args.start_block,
        batch_size: args.batch_size,
    };
    
    // Start the indexer in a background task unless --api-only flag is given
    if !args.api_only {
        task::spawn(async move {
            match start_indexer(config).await {
                Ok(_) => println!("Indexer completed successfully"),
                Err(e) => eprintln!("Indexer error: {:?}", e),
            }
        });
        
        println!("Indexer started in background");
    } else {
        println!("Running in API-only mode (indexer disabled)");
    }
    
    println!("Starting web server...");
    println!("API available at http://localhost:8000");
    
    // Initialize database
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    let pool = repositories::init_db(&database_url).await?;

    // AppState
    let app_state = models::AppState { db: pool.clone() };

    // Standard route
    #[get("/")]
    fn index() -> RawHtml<&'static str> {
        RawHtml(include_str!("../static/index.html"))
    }

    // Build Rocket instance
    let rocket = rocket::build()
        .mount("/", routes![index])
        .mount("/eth", routes![
            get_transfers
        ])
        .manage(app_state)
        .configure(rocket::Config::figment().merge(("json.pretty", true)));
    
    Ok(rocket)
} 