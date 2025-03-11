/// USDC token contract addresses
pub mod contracts {
    use web3::types::Address;
    use std::str::FromStr;

    /// USDC token contract address on Ethereum mainnet
    pub fn usdc_address() -> Address {
        Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")
            .expect("Invalid USDC contract address")
    }
} 