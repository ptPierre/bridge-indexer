# Bridge Indexer


A blockchain indexing service that tracks a token bridge between Holesky and Sepolia and provides an API to query the indexed data.

## Features

### Indexing Engine
- **Real-time ERC-20 Transfer Tracking**: Monitors token transfers as they happen
- **WebSocket Connection**: Maintains a persistent connection to the Ethereum network
- **Batch Processing**: Efficiently processes multiple transfers in batches for optimal database performance
- **Backfill Option**: Offers the possibility to start tracking historical data
- **ABI-based Decoding**: Uses the contract ABI to correctly parse event data

### API
- **RESTful Endpoints**: Clean API for retrieving transfer data
- **Pagination Support**: Efficient data retrieval with limit and page parameters
- **Formatted Response**: Structured JSON responses with metadata


### Technical Highlights
- **Rust Performance**: Built with Rust for speed and reliability
- **Asynchronous Processing**: Non-blocking I/O operations with Tokio runtime
- **PostgreSQL Storage**: Robust data persistence with postgres database
- **Rocket Web Server**: Fast and intuitive API framework

## Endpoints
1. Little API documentation
```
/
``` 
2. Getting USDC transfers
```
/bridge/events
```
## Project Structure

├── src/
│ ├── api/ # API endpoints
│ ├── models/ # Data models and database setup
│ ├── repositories/ # Database access layer
│ ├── services/ # Business logic (indexer)
│ ├── utils/ # Helper functions
│ ├── abis/ # Contract ABIs
│ ├── bin/ # Executable entry points
│ └── main.rs # Web server entry point
├── static/ # Static web assets
└── .env # Environment configuration
└── Cargo.toml # Dependencies

## Getting Started

### Prerequisites
- Install Rust and Cargo
- PostgreSQL database
- Ethereum API key (Alchemy, Infura, etc.)

### Create DB table
```
CREATE TABLE IF NOT EXISTS transfers (
    id SERIAL PRIMARY KEY,
    from_address TEXT NOT NULL,
    to_address TEXT NOT NULL,
    value TEXT NOT NULL,
    block_number BIGINT,
    tx_hash TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
```

### Configuration
Create a `.env` file following the .env.example file.

### Running
1. Start the live indexer and API:
   ```
   cargo run
   ```
2. Start indexer and API with backfill (live indexer starts automatically when historic data is filled):
   ```
   cargo run -- --days 3
   cargo run -- --start-block 16000000
   ```
3. For indexer only:
   ```
   cargo run --bin indexer
   ```
4. For indexer only with backfill:
   ```
   cargo run --bin indexer -- --days 3  # Backfills 3 days worth of blocks then continue with real-time
   cargo run --bin indexer -- --start-block 16000000  # Start from specific block
   ```
5. Run only API:
   ```
   cargo run -- --api-only
   ```

## Future Improvements
- Support for more ERC-20 tokens
- Transaction details and gas costs
- Offer a WebSocket API for real-time updates

