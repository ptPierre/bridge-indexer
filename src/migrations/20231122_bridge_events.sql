CREATE TABLE IF NOT EXISTS bridge_events (
    id SERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    network TEXT NOT NULL,
    token_address TEXT NOT NULL,
    from_address TEXT,
    to_address TEXT NOT NULL,
    amount TEXT NOT NULL,
    nonce BIGINT NOT NULL,
    block_number BIGINT,
    tx_hash TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS bridge_events_event_type_idx ON bridge_events(event_type);
CREATE INDEX IF NOT EXISTS bridge_events_network_idx ON bridge_events(network);
CREATE INDEX IF NOT EXISTS bridge_events_token_address_idx ON bridge_events(token_address);
CREATE INDEX IF NOT EXISTS bridge_events_from_address_idx ON bridge_events(from_address);
CREATE INDEX IF NOT EXISTS bridge_events_to_address_idx ON bridge_events(to_address);
CREATE INDEX IF NOT EXISTS bridge_events_block_number_idx ON bridge_events(block_number); 