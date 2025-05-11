-- Schema for blocks and files tables in SQLite database

-- Table to store blocks with their hash and data
CREATE TABLE IF NOT EXISTS blocks (
    hash VARCHAR(64) PRIMARY KEY,  -- SHA-256 hash of the block data (64 characters for hex representation)
    data BLOB NOT NULL,            -- The actual block data stored as a binary blob
    file_hash VARCHAR(64),         -- Hash of the file this block belongs to (NULL if not part of a file)
    block_index INTEGER,           -- The index of the block in the file (NULL if not part of a file)
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP  -- When the block was added to the database
);

-- Index on hash for faster lookups
CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks (hash);

-- Index on file_hash for faster lookups by file
CREATE INDEX IF NOT EXISTS idx_blocks_file_hash ON blocks (file_hash);

-- Table to store file metadata
CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- Auto-incrementing ID for the file
    file_name VARCHAR(255) NOT NULL,  -- Name of the file
    file_hash VARCHAR(64) NOT NULL,   -- SHA-256 hash of the file content (64 characters for hex representation)
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP  -- When the file was added to the database
);

-- Index on file_hash for faster lookups
CREATE INDEX IF NOT EXISTS idx_files_hash ON files (file_hash);
