CREATE TABLE IF NOT EXISTS transfers (
    id SERIAL PRIMARY KEY,
    from_address TEXT NOT NULL,
    to_address TEXT NOT NULL,
    value TEXT NOT NULL,
    block_number BIGINT,
    tx_hash TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS transfers_from_address_idx ON transfers(from_address);
CREATE INDEX IF NOT EXISTS transfers_to_address_idx ON transfers(to_address);
CREATE INDEX IF NOT EXISTS transfers_block_number_idx ON transfers(block_number);