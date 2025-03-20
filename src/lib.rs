use rocket::{get, routes};

pub mod api;
pub mod models;
pub mod services;
pub mod repositories;
pub mod utils;
use clap::Parser;
use eyre::Result;
use rocket::{Build, Rocket};
use api::bridge::get_bridge_events;
use tokio::task;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about = "Lobster - Bridge Event Indexer + API")]
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
    use services::bridge_indexer;

    println!("Starting application...");
    

    // Start the indexer in a background task unless --api-only flag is given
    if !args.api_only {
        task::spawn(async move {
            match bridge_indexer::start_bridge_indexer().await {
                Ok(_) => println!("Bridge indexer completed successfully"),
                Err(e) => eprintln!("Bridge indexer error: {:?}", e),
            }
        });
        
        println!("Bridge indexer started in background");
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
            get_bridge_events
        ])
        .manage(app_state)
        .configure(rocket::Config::figment().merge(("json.pretty", true)));
    
    Ok(rocket)
} 