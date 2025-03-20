-- Add new columns for swap events if they don't exist
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='bridge_events' AND column_name='source_token') THEN
        ALTER TABLE bridge_events ADD COLUMN source_token VARCHAR(42);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='bridge_events' AND column_name='target_token') THEN
        ALTER TABLE bridge_events ADD COLUMN target_token VARCHAR(42);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='bridge_events' AND column_name='target_amount') THEN
        ALTER TABLE bridge_events ADD COLUMN target_amount VARCHAR;
    END IF;
END
$$; 