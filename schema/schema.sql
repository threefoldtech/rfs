-- inode table and main entrypoint of the schema
CREATE TABLE IF NOT EXISTS inode (
    ino INTEGER PRIMARY KEY AUTOINCREMENT,
    parent INTEGER,
    name VARCHAR(255),
    size INTEGER,
    uid INTEGER,
    gid INTEGER,
    mode INTEGER,
    rdev INTEGER,
    ctime INTEGER,
    mtime INTEGER
);

CREATE INDEX IF NOT EXISTS parents ON inode (parent);

-- extra data for each inode for special types (say link targets)
CREATE TABLE IF NOT EXISTS extra (
    ino INTEGER PRIMARY KEY,
    data VARCHAR(4096)
);

-- blocks per file, order of insertion is important
CREATE TABLE IF NOT EXISTS block (
    ino INTEGER,
    hash VARCHAR(16),
    key VARCHAR(16)
);

CREATE INDEX IF NOT EXISTS block_ino ON block (ino);

-- global flist tags, this can include values like `version`, `description`, `block-size`, etc..
-- it can also hold extra user-defined tags for extensions
CREATE TABLE IF NOT EXISTS tag (
    key VARCHAR(10) PRIMARY KEY,
    value VARCHAR(255)
);

-- routing table define ranges where blobs can be found. This allows "sharding" by be able to retrieve
-- blobs from different partitions using the prefix range (hashes that are )
CREATE TABLE IF NOT EXISTS route (
    start integer, -- one byte hash prefix
    end integer, -- one byte hash prefix
    url VARCHAR(2048)
);
