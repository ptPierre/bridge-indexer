/// Bridge contract addresses
pub mod contracts {
    use web3::types::Address;
    use std::str::FromStr;
    
    /// Bridge contract address on Sepolia
    pub fn sepolia_bridge_address() -> Address {
        Address::from_str("0xC57ef84129ee3d73d558c2AE69503060e328d494")
            .expect("Invalid Sepolia bridge address")
    }
    
    /// Bridge contract address on Holesky
    pub fn holesky_bridge_address() -> Address {
        Address::from_str("0x1533600886E59FD9FC1Af1c801C38D4dD9582935")
            .expect("Invalid Holesky bridge address")
    }
}

/// Network configurations
pub mod networks {
    /// Get the RPC URL for a specific network
    pub fn get_rpc_url(network: &str) -> String {
        match std::env::var(format!("{}_RPC_URL", network.to_uppercase())) {
            Ok(url) => url,
            Err(_) => panic!("{}_RPC_URL environment variable not set", network.to_uppercase()),
        }
    }
} 