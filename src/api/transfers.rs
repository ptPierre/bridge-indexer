use rocket::State;
use crate::models::transfers::Transfer;
use crate::models::AppState;
use crate::repositories::transfers as transfers_repo;
use rocket::get;
use serde::Serialize;
use crate::utils::ethereum::{get_token_info, TokenInfo};
use serde_json;
use crate::utils::config::contracts;
// API response structure
#[derive(Serialize)]
pub struct TransferResponse {
    // Add metadata
    total: usize,
    page: usize,
    limit: u64,
    token: TokenInfo,
    transfers: Vec<FormattedTransfer>,
}

// Formatted transfer JSON response
#[derive(Serialize)]
struct FormattedTransfer {
    sender: String,
    recipient: String,
    amount: String,
    block_number: String,
    tx_hash: Option<String>,
}

impl From<Transfer> for FormattedTransfer {
    fn from(transfer: Transfer) -> Self {
        // Clean up address format
        let sender = transfer.from_address.replace("\"", "");
        let recipient = transfer.to_address.replace("\"", "");
        
        // Convert BigDecimal to String for the API response
        let amount = transfer.value.to_string();
        
        let block_number = transfer.block_number
            .map(|bn| bn.to_string())
            .unwrap_or_else(|| "0".to_string());
        
        FormattedTransfer {
            sender,
            recipient,
            amount,
            block_number,
            tx_hash: transfer.tx_hash,
        }
    }
}

#[get("/transfers?<limit>&<page>")]
pub async fn get_transfers(
    state: &State<AppState>, 
    limit: Option<u64>, 
    page: Option<u64>
) -> rocket::response::content::RawJson<String> {
    // Default params
    let limit = limit.unwrap_or(10);
    let page = page.unwrap_or(1);
    let offset = ((page - 1) * limit) as i64;
    
    // Get token info for USDC
    let usdc_address = format!("{:?}", contracts::usdc_address());
    let token_info = match get_token_info(&usdc_address).await {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Error fetching token info: {:?}", e);
            // Fallback 
            TokenInfo {
                decimals: 6,
                symbol: "USDC".to_string(),
            }
        }
    };
    
    match transfers_repo::get_transfers(&state.db, Some(limit as i64), offset).await {
        Ok(transfers) => {
            let total = transfers.len();
            let formatted: Vec<FormattedTransfer> = transfers
                .into_iter()
                .map(FormattedTransfer::from)
                .collect();
            
            let response_data = TransferResponse {
                total,
                page: page as usize,
                limit,
                token: token_info,
                transfers: formatted,
            };
            
            // Pretty-print the JSON (just makes it look cooler)
            let json_string = serde_json::to_string_pretty(&response_data).unwrap_or_default();
            rocket::response::content::RawJson(json_string)
        },
        Err(e) => {
            eprintln!("Error fetching transfers: {:?}", e);
            let response_data = TransferResponse {
                total: 0,
                page: page as usize,
                limit,
                token: token_info,
                transfers: vec![],
            };
            
            let json_string = serde_json::to_string_pretty(&response_data).unwrap_or_default();
            rocket::response::content::RawJson(json_string)
        }
    }
}

