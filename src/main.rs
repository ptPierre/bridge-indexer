pub mod api;
pub mod models;
pub mod services;
pub mod repositories;
pub mod utils;


use dotenv::dotenv;
use eyre::Result;
use lobster::{start_app, AppArgs};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = AppArgs::parse();
    
    // Load environment variables
    dotenv().ok();
    
    // Start the application
    let rocket = start_app(args).await?;
    
    // Launch the web server
    let _ = rocket.launch().await?;
    
    Ok(())
}