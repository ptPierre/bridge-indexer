use std::fs;
use std::path::Path;
use eyre::Result;
use web3::ethabi::Contract;

pub fn load_abi<P: AsRef<Path>>(path: P) -> Result<Contract> {
    let file = fs::read_to_string(path)?;
    let contract = Contract::load(file.as_bytes())?;
    Ok(contract)
} 