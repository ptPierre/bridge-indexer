// File can be empty or removed entirely if not used elsewhere 

use web3::{
    types::Address,
    Web3,
};
use web3::contract::{Contract, Options};
use web3::transports::Http;
use std::str::FromStr;
use std::env;
use eyre::Result;
use serde::Serialize;
use std::sync::RwLock;
use once_cell::sync::Lazy;
use crate::utils::abi::load_abi;
use std::path::Path;

#[derive(Clone, Serialize)]
pub struct TokenInfo {
    pub decimals: u8,
    pub symbol: String,
}

// Cache the token info 
static TOKEN_CACHE: Lazy<RwLock<Option<TokenInfo>>> = Lazy::new(|| RwLock::new(None));

// Get token info
pub async fn get_token_info(contract_address: &str) -> Result<TokenInfo> {
    // Check cache first
    if let Some(info) = TOKEN_CACHE.read().unwrap().clone() {
        return Ok(info);
    }
    
    // If no cached data I fetch from the contract
    let http_url = env::var("HTTP_RPC_URL")
        .expect("HTTP_RPC_URL must be set in .env file");
    
    let transport = Http::new(&http_url)?;
    let web3 = Web3::new(transport);
    
    // Load the ABI from file
    let abi_path = Path::new("src/abis/erc20.json");
    let contract_abi = load_abi(abi_path)?;
    
    // Create contract instance
    let address = Address::from_str(contract_address)?;
    let contract = Contract::new(web3.eth(), address, contract_abi);
    
    // Calling decimals()
    let decimals: u8 = contract.query("decimals", (), None, Options::default(), None).await?;
    
    // Calling symbol()
    let symbol: String = contract.query("symbol", (), None, Options::default(), None).await?;
    
    // Create and cache token info
    let token_info = TokenInfo { decimals, symbol };
    *TOKEN_CACHE.write().unwrap() = Some(token_info.clone());
    
    Ok(token_info)
} 