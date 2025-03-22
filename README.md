# Bridge Indexer


A blockchain indexing service that tracks a token bridge between Holesky and Sepolia and provides an API to query the indexed data.


## Endpoints
1. Little API documentation
```
/
``` 
2. Getting all the events detected by the indexer
```
/bridge/events
```

## Getting Started

### Prerequisites
- Install Rust and Cargo
- PostgreSQL database
- Ethereum API key (Alchemy, Infura, etc.)

### Create DB table

Run the SQL queries in the migration files on your postgres database.
1. bridge_events
2. update_bridge_events

### Configuration
Create a `.env` file following the .env.example file.

### Running
Start the live indexer and API:
   ```
   cargo run
   ```


## Contracts

The two bridge contracts that are indexed here are 
1. Holesky Bridge
```
0x1533600886E59FD9FC1Af1c801C38D4dD9582935
```

2. Sepolia Bridge
```
0xC57ef84129ee3d73d558c2AE69503060e328d494
```

The bridge can be used with every token, as long as the contracts are funded appropriately.

The implemenation of the contrcats can be found here:
[Contracts](https://github.com/ptPierre/bridge-contracts)
