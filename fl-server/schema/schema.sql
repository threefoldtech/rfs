-- Schema for blocks and files metadata relation tables in SQLite database

-- Table to store file metadata
CREATE TABLE IF NOT EXISTS metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- Auto-incrementing ID for the file
    file_hash VARCHAR(64) NOT NULL,   -- SHA-256 hash of the file content (64 characters for hex representation)
    block_index INTEGER,           -- The index of the block in the file (NULL if not part of a file)
    block_hash VARCHAR(64),         -- SHA-256 hash of the block data (64 characters for hex representation)
    user_id INTEGER,               -- ID of the user who uploaded the block
    block_size INTEGER,            -- Size of the block in bytes
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,  -- When the file was added to the database
    FOREIGN KEY (user_id) REFERENCES users(id)  -- Foreign key constraint to users table
);

-- Index on file_hash for faster lookups
CREATE INDEX IF NOT EXISTS idx_files_hash ON metadata (file_hash);

-- Table to store user information
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- Auto-incrementing ID for the user
    username VARCHAR(255) NOT NULL UNIQUE, -- Unique username
    password VARCHAR(255) NOT NULL,        -- Hashed password
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP -- When the user was added to the database
);

-- Index on username for faster lookups
CREATE INDEX IF NOT EXISTS idx_users_username ON users (username);
