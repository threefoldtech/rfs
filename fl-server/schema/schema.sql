-- Schema for blocks and files metadata relation tables in SQLite database

-- Table to store file metadata
CREATE TABLE IF NOT EXISTS metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- Auto-incrementing ID for the file
    file_hash VARCHAR(64) NOT NULL,   -- SHA-256 hash of the file content (64 characters for hex representation)
    block_index INTEGER,           -- The index of the block in the file (NULL if not part of a file)
    block_hash VARCHAR(64),         -- SHA-256 hash of the block data (64 characters for hex representation)
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP  -- When the file was added to the database
);

-- Index on file_hash for faster lookups
CREATE INDEX IF NOT EXISTS idx_files_hash ON metadata (file_hash);
