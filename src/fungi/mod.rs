use std::path::Path;

use sqlx::{sqlite::SqliteRow, FromRow, Row, SqlitePool};

const HASH_LEN: usize = 16;
const KEY_LEN: usize = 16;

static SCHEMA: &'static str = include_str!("../../schema/schema.sql");

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to execute query: {0}")]
    SqlError(#[from] sqlx::Error),

    #[error("invalid hash length")]
    InvalidHash,

    #[error("invalid key length")]
    InvalidKey,

    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
}

pub type Ino = u64;
pub type Mode = u32;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Default)]
pub struct Inode {
    pub ino: Ino,
    pub parent: Ino,
    pub name: String,
    pub size: usize,
    pub uid: u32,
    pub gid: u32,
    pub mode: Mode,
    pub rdev: u32,
    pub ctime: i64,
    pub mtime: i64,
    pub data: Option<Vec<u8>>,
}

impl FromRow<'_, SqliteRow> for Inode {
    fn from_row(row: &'_ SqliteRow) -> std::result::Result<Self, sqlx::Error> {
        Ok(Self {
            ino: row.get::<i64, &str>("ino") as Ino,
            parent: row.get::<i64, &str>("parent") as Ino,
            name: row.get("name"),
            size: row.get::<i64, &str>("size") as usize,
            uid: row.get("uid"),
            gid: row.get("uid"),
            mode: row.get("mode"),
            rdev: row.get("rdev"),
            ctime: row.get("ctime"),
            mtime: row.get("mtime"),
            data: row.get("data"),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Block {
    pub hash: [u8; HASH_LEN],
    pub key: [u8; KEY_LEN],
}

impl FromRow<'_, SqliteRow> for Block {
    fn from_row(row: &'_ SqliteRow) -> std::result::Result<Self, sqlx::Error> {
        let hash: &[u8] = row.get("hash");
        if hash.len() != HASH_LEN {
            return Err(sqlx::Error::Decode(Box::new(Error::InvalidHash)));
        }

        let key: &[u8] = row.get("key");

        if hash.len() != KEY_LEN {
            return Err(sqlx::Error::Decode(Box::new(Error::InvalidKey)));
        }

        let mut block = Self::default();
        block.hash.copy_from_slice(hash);
        block.key.copy_from_slice(key);

        Ok(block)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Route {
    pub start: u8,
    pub end: u8,
    pub url: String,
}

impl FromRow<'_, SqliteRow> for Route {
    fn from_row(row: &'_ SqliteRow) -> std::result::Result<Self, sqlx::Error> {
        Ok(Self {
            start: row.get("start"),
            end: row.get("end"),
            url: row.get("url"),
        })
    }
}

#[derive(Debug, Clone)]
pub enum Tag<'a> {
    Version,
    Description,
    Author,
    Custom(&'a str),
}

impl<'a> Tag<'a> {
    fn key(&self) -> &str {
        match self {
            Self::Version => "version",
            Self::Description => "description",
            Self::Author => "author",
            Self::Custom(a) => a,
        }
    }
}

#[derive(Clone)]
pub struct Reader {
    pool: SqlitePool,
}

impl Reader {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let con = format!("sqlite://{}", path.as_ref().to_str().unwrap());
        let pool = SqlitePool::connect(&con).await?;

        Ok(Self { pool })
    }

    pub async fn inode(&self, ino: Ino) -> Result<Inode> {
        let inode: Inode = sqlx::query_as(r#"select inode.*, extra.data
                                                    from inode left join extra on inode.ino = extra.ino
                                                    where inode.ino = ?;"#)
                    .bind(ino as i64).fetch_one(&self.pool).await?;

        Ok(inode)
    }

    pub async fn children(&self, parent: Ino, limit: u32, offset: u32) -> Result<Vec<Inode>> {
        let results: Vec<Inode> = sqlx::query_as(
            r#"select inode.*, extra.data
                                from inode left join extra on inode.ino = extra.ino
                                where inode.parent = ? limit ? offset ?;"#,
        )
        .bind(parent as i64)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }

    pub async fn blocks(&self, ino: Ino) -> Result<Vec<Block>> {
        let results: Vec<Block> = sqlx::query_as("select hash, key from block where ino = ?;")
            .bind(ino as i64)
            .fetch_all(&self.pool)
            .await?;

        Ok(results)
    }

    pub async fn tag(&self, tag: Tag<'_>) -> Result<Option<String>> {
        let value: Option<(String,)> = sqlx::query_as("select value from tag where key = ?;")
            .bind(tag.key())
            .fetch_optional(&self.pool)
            .await?;

        Ok(value.map(|v| v.0))
    }

    pub async fn routes(&self) -> Result<Vec<Route>> {
        let results: Vec<Route> = sqlx::query_as("select start, end, url from route;")
            .fetch_all(&self.pool)
            .await?;

        Ok(results)
    }
}

#[derive(Clone)]
pub struct Writer {
    pool: SqlitePool,
}

impl Writer {
    /// create a new mkondo writer
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        tokio::fs::OpenOptions::default()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&path)
            .await?;

        let con = format!("sqlite://{}", path.as_ref().to_str().unwrap());
        let pool = SqlitePool::connect(&con).await?;

        sqlx::query(SCHEMA).execute(&pool).await?;

        Ok(Self { pool })
    }

    /// inode add an inode to the flist
    pub async fn inode(&self, inode: Inode) -> Result<Ino> {
        let result = sqlx::query(
            r#"insert into inode (parent, name, size, uid, gid, mode, rdev, ctime, mtime)
                                       values (?, ?, ?, ?, ?, ?, ?, ?, ?);"#,
        )
        .bind(inode.parent as i64)
        .bind(inode.name)
        .bind(inode.size as i64)
        .bind(inode.uid)
        .bind(inode.gid)
        .bind(inode.mode)
        .bind(inode.rdev)
        .bind(inode.ctime)
        .bind(inode.mtime)
        .execute(&self.pool)
        .await?;

        let ino = result.last_insert_rowid() as Ino;
        if let Some(data) = &inode.data {
            sqlx::query("insert into extra(ino, data) values (?, ?)")
                .bind(ino as i64)
                .bind(data)
                .execute(&self.pool)
                .await?;
        }

        Ok(ino)
    }

    pub async fn block(&self, ino: Ino, hash: &[u8; HASH_LEN], key: &[u8; KEY_LEN]) -> Result<()> {
        sqlx::query("insert into block (ino, hash, key) values (?, ?, ?)")
            .bind(ino as i64)
            .bind(&hash[..])
            .bind(&key[..])
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn route<U: AsRef<str>>(&self, start: u8, end: u8, url: U) -> Result<()> {
        sqlx::query("insert into route (start, end, url) values (?, ?, ?)")
            .bind(start)
            .bind(end)
            .bind(url.as_ref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn tag<V: AsRef<str>>(&self, tag: Tag<'_>, value: V) -> Result<()> {
        sqlx::query("insert into tag (key, value) values (?, ?);")
            .bind(tag.key())
            .bind(value.as_ref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_inode() {
        const PATH: &str = "/tmp/inode.fl";
        let meta = Writer::new(PATH).await.unwrap();

        let ino = meta
            .inode(Inode {
                name: "/".into(),
                data: Some("target".into()),
                ..Inode::default()
            })
            .await
            .unwrap();

        assert_eq!(ino, 1);

        let meta = Reader::new(PATH).await.unwrap();
        let inode = meta.inode(ino).await.unwrap();

        assert_eq!(inode.name, "/");
        assert!(inode.data.is_some());
        assert_eq!(inode.data.unwrap().as_slice(), "target".as_bytes());
    }

    #[tokio::test]
    async fn test_get_children() {
        const PATH: &str = "/tmp/children.fl";
        let meta = Writer::new(PATH).await.unwrap();

        let ino = meta
            .inode(Inode {
                name: "/".into(),
                data: Some("target".into()),
                ..Inode::default()
            })
            .await
            .unwrap();

        for name in ["bin", "etc", "usr"] {
            meta.inode(Inode {
                parent: ino,
                name: name.into(),
                ..Inode::default()
            })
            .await
            .unwrap();
        }
        let meta = Reader::new(PATH).await.unwrap();
        let children = meta.children(ino, 10, 0).await.unwrap();

        assert_eq!(children.len(), 3);
        assert_eq!(children[0].name, "bin");
    }

    #[tokio::test]
    async fn test_get_block() {
        const PATH: &str = "/tmp/block.fl";
        let meta = Writer::new(PATH).await.unwrap();
        let hash: [u8; HASH_LEN] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let key1: [u8; KEY_LEN] = [1; KEY_LEN];
        let key2: [u8; KEY_LEN] = [2; KEY_LEN];

        meta.block(1, &hash, &key1).await.unwrap();
        meta.block(1, &hash, &key2).await.unwrap();

        let meta = Reader::new(PATH).await.unwrap();

        let blocks = meta.blocks(1).await.unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].hash, hash);
        assert_eq!(blocks[0].key, key1);
        assert_eq!(blocks[1].key, key2);
    }

    #[tokio::test]
    async fn test_get_tag() {
        const PATH: &str = "/tmp/tag.fl";
        let meta = Writer::new(PATH).await.unwrap();
        meta.tag(Tag::Version, "0.1").await.unwrap();
        meta.tag(Tag::Author, "azmy").await.unwrap();
        meta.tag(Tag::Custom("custom"), "value").await.unwrap();

        let meta = Reader::new(PATH).await.unwrap();

        assert!(matches!(
            meta.tag(Tag::Version).await.unwrap().as_deref(),
            Some("0.1")
        ));

        assert!(matches!(
            meta.tag(Tag::Custom("custom")).await.unwrap().as_deref(),
            Some("value")
        ));

        assert!(matches!(
            meta.tag(Tag::Custom("unknown")).await.unwrap(),
            None
        ));
    }

    #[tokio::test]
    async fn test_get_routes() {
        const PATH: &str = "/tmp/route.fl";
        let meta = Writer::new(PATH).await.unwrap();
        let hash: [u8; HASH_LEN] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let key1: [u8; KEY_LEN] = [1; KEY_LEN];
        let key2: [u8; KEY_LEN] = [2; KEY_LEN];

        meta.block(1, &hash, &key1).await.unwrap();
        meta.block(1, &hash, &key2).await.unwrap();

        let meta = Reader::new(PATH).await.unwrap();

        let blocks = meta.blocks(1).await.unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].hash, hash);
        assert_eq!(blocks[0].key, key1);
        assert_eq!(blocks[1].key, key2);
    }
}
