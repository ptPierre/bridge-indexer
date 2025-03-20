use serde::{Serialize, Deserialize};
use web3::types::{Address, U256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeEventType {
    Deposit,
    Distribution,
}

impl ToString for BridgeEventType {
    fn to_string(&self) -> String {
        match self {
            BridgeEventType::Deposit => "Deposit".to_string(),
            BridgeEventType::Distribution => "Distribution".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeEvent {
    pub id: Option<i32>,
    pub event_type: String,
    pub network: String,
    pub token_address: String,
    pub from_address: Option<String>,
    pub to_address: String,
    pub amount: String,
    pub nonce: i64,
    pub block_number: Option<i64>,
    pub tx_hash: Option<String>,
    pub source_token: Option<String>,
    pub target_token: Option<String>,
    pub target_amount: Option<String>,
}

impl BridgeEvent {
    pub fn new_deposit(
        network: &str,
        token: Address,
        from: Address,
        to: Address,
        amount: U256,
        nonce: U256,
        block_number: Option<u64>,
        tx_hash: Option<String>,
    ) -> eyre::Result<Self> {
        Ok(Self {
            id: None,
            event_type: BridgeEventType::Deposit.to_string(),
            network: network.to_string(),
            token_address: format!("{:?}", token),
            from_address: Some(format!("{:?}", from)),
            to_address: format!("{:?}", to),
            amount: amount.to_string(),
            nonce: nonce.as_u64() as i64,
            block_number: block_number.map(|bn| bn as i64),
            tx_hash,
            source_token: None,
            target_token: None,
            target_amount: None,
        })
    }

    pub fn new_distribution(
        network: &str,
        token: Address,
        to: Address,
        amount: U256,
        nonce: U256,
        block_number: Option<u64>,
        tx_hash: Option<String>,
    ) -> eyre::Result<Self> {
        Ok(Self {
            id: None,
            event_type: BridgeEventType::Distribution.to_string(),
            network: network.to_string(),
            token_address: format!("{:?}", token),
            from_address: None,
            to_address: format!("{:?}", to),
            amount: amount.to_string(),
            nonce: nonce.as_u64() as i64,
            block_number: block_number.map(|bn| bn as i64),
            tx_hash,
            source_token: None,
            target_token: None,
            target_amount: None,
        })
    }

    pub fn new_swap_deposit(
        network: &str,
        source_token: Address,
        target_token: Address,
        from: Address,
        to: Address,
        source_amount: U256,
        nonce: U256,
        block_number: Option<u64>,
        tx_hash: Option<String>,
    ) -> eyre::Result<Self> {
        Ok(Self {
            id: None,
            event_type: "SwapDeposit".to_string(),
            network: network.to_string(),
            token_address: format!("{:?}", source_token),
            from_address: Some(format!("{:?}", from)),
            to_address: format!("{:?}", to),
            amount: source_amount.to_string(),
            nonce: nonce.as_u64() as i64,
            block_number: block_number.map(|bn| bn as i64),
            tx_hash,
            source_token: Some(format!("{:?}", source_token)),
            target_token: Some(format!("{:?}", target_token)),
            target_amount: None,
        })
    }
    
    pub fn new_swap_distribution(
        network: &str,
        source_token: Address,
        target_token: Address,
        to: Address,
        source_amount: U256,
        target_amount: U256,
        nonce: U256,
        block_number: Option<u64>,
        tx_hash: Option<String>,
    ) -> eyre::Result<Self> {
        Ok(Self {
            id: None,
            event_type: "SwapDistribution".to_string(),
            network: network.to_string(),
            token_address: format!("{:?}", target_token),
            from_address: None,
            to_address: format!("{:?}", to),
            amount: source_amount.to_string(),
            nonce: nonce.as_u64() as i64,
            block_number: block_number.map(|bn| bn as i64),
            tx_hash,
            source_token: Some(format!("{:?}", source_token)),
            target_token: Some(format!("{:?}", target_token)),
            target_amount: Some(target_amount.to_string()),
        })
    }
} 