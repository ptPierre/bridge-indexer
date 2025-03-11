use web3::types::{Address, U256};
use serde::{Serialize, Deserialize};


// Just a data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: Option<i32>,
    pub from_address: String,
    pub to_address: String,
    pub value: String,
    pub block_number: Option<i64>,
    pub tx_hash: Option<String>,
}

impl Transfer {
    pub fn new(from: Address, to: Address, value: U256, block_number: Option<u64>, tx_hash: Option<String>) -> eyre::Result<Self> {
        Ok(Self {
            id: None,
            from_address: format!("{:?}", from),
            to_address: format!("{:?}", to),
            value: value.to_string(),
            block_number: block_number.map(|bn| bn as i64),
            tx_hash
        })
    }
}

