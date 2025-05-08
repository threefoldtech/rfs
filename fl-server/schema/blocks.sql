-- Schema for blocks table in SQLite database

-- Table to store blocks with their hash and data
CREATE TABLE IF NOT EXISTS blocks (
    hash VARCHAR(64) PRIMARY KEY,  -- SHA-256 hash of the block data (64 characters for hex representation)
    data BLOB NOT NULL,            -- The actual block data stored as a binary blob
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP  -- When the block was added to the database
);

-- Index on hash for faster lookups
CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks (hash);