use rocket::serde::json::{Json, Value};
use rocket::State;
use rocket::serde::json::serde_json::json;
use crate::models::AppState;
use crate::repositories::bridge as bridge_repo;

#[rocket::get("/bridge/events?<event_type>&<network>&<limit>&<page>")]
pub async fn get_bridge_events(
    event_type: Option<String>,
    network: Option<String>,
    limit: Option<u64>,
    page: Option<u64>,
    state: &State<crate::models::AppState>
) -> Json<Value> {
    // Default values
    let limit_val = limit.unwrap_or(10).min(100) as i64;
    let page_val = page.unwrap_or(1).max(1) as i64;
    let offset = (page_val - 1) * limit_val;
    
    // Get events from repository
    match bridge_repo::get_bridge_events(&state.db, event_type, network, Some(limit_val), offset).await {
        Ok(events) => {
            // Format response
            Json(json!({
                "total": events.len(),
                "page": page_val,
                "limit": limit_val,
                "events": events.iter().map(|event| {
                    let mut event_json = json!({
                        "event_type": event.event_type,
                        "network": event.network,
                        "token": event.token_address,
                        "to": event.to_address,
                        "amount": format!("{} tokens", event.amount),
                        "nonce": event.nonce,
                        "block_number": event.block_number,
                        "tx_hash": event.tx_hash
                    });
                    
                    if let Some(from) = &event.from_address {
                        event_json.as_object_mut().unwrap().insert("from".to_string(), json!(from));
                    }
                    
                    event_json
                }).collect::<Vec<_>>()
            }))
        },
        Err(e) => {
            eprintln!("Error fetching bridge events: {:?}", e);
            Json(json!({
                "error": "Failed to fetch bridge events",
                "details": format!("{:?}", e)
            }))
        }
    }
} 