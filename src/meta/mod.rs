pub mod inode;
pub mod types;

use anyhow::{Context, Result};
use blake2::digest::{Update, VariableOutput};
use blake2::VarBlake2b;
use flate2::read::GzDecoder;
use inode::Inode;
use sqlx::sqlite::SqlitePool;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;
use tar::Archive;
use tokio::sync::Mutex;
use types::{Entry, EntryKind};

#[derive(Error, Debug)]
pub enum MetaError {
    #[error("error not found")]
    EntryNotFound,
}

pub struct Hash(Vec<u8>);
impl Hash {
    pub fn new(w: &str) -> Hash {
        let mut hasher = VarBlake2b::new(16).unwrap();

        hasher.update(w.as_bytes());
        let mut result: Vec<u8> = Vec::new();
        hasher.finalize_variable(|res| {
            result = res.to_vec();
        });

        Hash(result)
    }

    fn hex(&self) -> String {
        let mut result = String::new();
        for i in self.0.as_slice() {
            write!(&mut result, "{:02x}", i).unwrap();
        }
        result
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.hex())
    }
}

#[derive(Clone, Debug)]
pub struct Metadata {
    pool: SqlitePool,
    mask: inode::Mask,
    lru: Arc<Mutex<lru::LruCache<String, Arc<types::Entry>>>>,
    acis: Arc<Mutex<lru::LruCache<String, types::Aci>>>,
}

impl Metadata {
    // new creates a new metadata given a .sqlite3 db file
    async fn new<P: AsRef<Path>>(p: P) -> Result<Metadata> {
        let con = format!("sqlite://{}", p.as_ref().to_str().unwrap());
        let pool = SqlitePool::connect(&con)
            .await
            .context("failed to open metadata database")?;

        let (max,): (i64,) = sqlx::query_as("select max(rowid) from entries")
            .fetch_one(&pool)
            .await?;

        let mask = inode::Mask::from(max as u64);
        let lru = Arc::new(Mutex::new(lru::LruCache::new(512)));
        let acis = Arc::new(Mutex::new(lru::LruCache::new(10)));
        Ok(Metadata {
            pool,
            mask,
            lru,
            acis,
        })
    }

    pub async fn open<P: AsRef<Path>>(p: P) -> Result<Metadata> {
        let p = p.as_ref();
        if !p.exists() {
            bail!("provided metadata path does not exist");
        }

        if p.is_dir() {
            return Self::new(p.join("flistdb.sqlite3")).await;
        }

        let ext = match p.extension() {
            Some(ext) => ext,
            None => bail!("unknown metadata file"),
        };

        if ext == "sqlite3" {
            return Self::new(p).await;
        } else {
            // extract the flist
            // create directory for extracted meta
            let dir = p.with_file_name(format!("{}.d", p.file_name().unwrap().to_str().unwrap()));
            std::fs::create_dir_all(&dir)?;
            let tar_gz = std::fs::File::open(p)?;
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);
            archive.unpack(&dir)?;

            return Self::new(dir.join("flistdb.sqlite3")).await;
        }
    }

    pub async fn root(&self) -> Result<Arc<types::Entry>> {
        let root = Hash::new("").hex();
        return self.dir_by_key(root).await;
    }

    fn inode(&self, ino: u64) -> Inode {
        Inode::new(self.mask, ino)
    }

    pub async fn entry(&self, i: u64) -> Result<Entry> {
        let inode = self.inode(i);
        let (dir_ino, index) = inode.split();
        let dir = self.dir_by_inode(dir_ino).await?;
        if index == 0 {
            return Ok((*dir).clone());
        }

        let dir_kind = match &dir.kind {
            EntryKind::Dir(dir) => dir,
            _ => bail!("invalid directory kind"),
        };

        let index = index as usize - 1;
        if index >= dir_kind.entries.len() {
            bail!(MetaError::EntryNotFound);
        }

        let entry = &dir_kind.entries[index];
        if let EntryKind::SubDir(ref sub) = entry.kind {
            let dir = self.dir_by_key(&sub.key).await?;
            // probably need to be cached
            return Ok((*dir).clone());
        }

        Ok(entry.clone())
    }

    async fn dir_by_key<S: AsRef<str>>(&self, key: S) -> Result<Arc<types::Entry>> {
        // let root = Hash::new("").hex();
        let mut lru = self.lru.lock().await;
        if let Some(dir) = lru.get(key.as_ref()) {
            return Ok(dir.clone());
        }

        let (id, data): (i64, Vec<u8>) =
            sqlx::query_as("select rowid, value from entries where key = ?")
                .bind(key.as_ref())
                .fetch_one(&self.pool)
                .await
                .context("failed to find directory")?;
        let id = id as u64;

        // that's only place where we create a directory
        // so we can cache it in lru now.
        let dir = types::Dir::from(key.as_ref(), inode::Inode::new(self.mask, id), data)?;
        let dir = Arc::new(dir);
        lru.put(key.as_ref().into(), dir.clone());

        Ok(dir)
    }

    async fn dir_by_inode(&self, ino: u64) -> Result<Arc<types::Entry>> {
        if ino == 1 {
            return self.root().await;
        }

        let (key,): (String,) = sqlx::query_as("select key from entries where rowid = ?")
            .bind(ino as i64)
            .fetch_one(&self.pool)
            .await
            .context("failed to find directory")?;

        self.dir_by_key(key).await
    }

    pub async fn dir_inode<S: AsRef<str>>(&self, key: S) -> Result<Inode> {
        let (id,): (i64,) = sqlx::query_as("select rowid from entries where key = ?")
            .bind(key.as_ref())
            .fetch_one(&self.pool)
            .await?;

        Ok(Inode::new(self.mask, id as u64))
    }

    async fn aci<S: AsRef<str>>(&self, key: S) -> Result<types::Aci> {
        let mut acis = self.acis.lock().await;
        if let Some(aci) = acis.get(key.as_ref()) {
            return Ok(aci.clone());
        }
        let (data,): (Vec<u8>,) = sqlx::query_as("select value from entries where key = ?")
            .bind(key.as_ref())
            .fetch_one(&self.pool)
            .await
            .context("failed to find directory")?;

        // that's only place where we create a directory
        // so we can cache it in lru now.
        let aci = types::Aci::new(data)?;
        acis.put(key.as_ref().into(), aci.clone());
        Ok(aci)
    }
}
