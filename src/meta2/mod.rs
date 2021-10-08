use anyhow::Result;
use blake2::digest::VariableOutput;
use blake2::VarBlake2b;
use sqlx::sqlite;
use std::fmt::{Error, Write};
pub mod inode;
pub use inode::{Inode, Mask};

pub mod types;
pub use types::{Either, EntryKind};

struct Hash(Vec<u8>);
impl Hash {
    fn new(w: &str) -> Hash {
        let mut hasher = VarBlake2b::new(16).unwrap();
        hasher.input(w);

        Hash(hasher.vec_result())
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
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), Error> {
        write!(f, "{}", self.hex())
    }
}

impl sqlite::Bindable for Hash {
    fn bind(self, stmt: &mut Statement, i: usize) -> sqlite::Result<()> {
        stmt.bind(i, format!("{}", self).as_str())
    }
}

pub struct Manager {
    con: sqlite::SqlitePool,
    mask: Mask,
}

impl Manager {
    pub async fn new<T: AsRef<std::path::Path>>(path: T) -> Result<Manager> {
        let pool = sqlite::SqlitePool::connect(path).await?;
        let mask = Self::get_inode_mask(&pool).await?;

        Ok(Manager {
            con: pool,
            mask: mask,
        })
    }

    pub fn get_inode(&self, ino: u64) -> Inode {
        Inode::new(self.mask, ino)
    }

    async fn get_inode_mask(con: &sqlite::SqlitePool) -> Result<inode::Mask> {
        let max: (u64,) = sqlx::query_as("select max(rowid) from entries")
            .fetch_one(&con)
            .await?;
        Ok(inode::Mask::from(max))
    }

    fn dir_inode_from_key(&self, key: &str) -> Result<Inode> {
        let mut stmt = self
            .con
            .prepare("select rowid from entries where key = ?")?;

        stmt.bind(1, key)?;
        if let sqlite::State::Row = stmt.next()? {
            let id: u64 = stmt.read::<i64>(0)? as u64;
            Ok(Inode::new(self.mask, id))
        } else {
            bail!("not found")
        }
    }

    pub fn get_dir_by_key(&self, key: &str) -> Result<types::Dir> {
        let mut stmt = self
            .con
            .prepare("select rowid, value from entries where key = ?")?;
        stmt.bind(1, key)?;

        if let sqlite::State::Row = stmt.next()? {
            let id: u64 = stmt.read::<i64>(0)? as u64;
            let bytes: Vec<u8> = stmt.read(1)?;
            Ok(types::Dir::new(&self, &bytes, Inode::new(self.mask, id))?)
        } else {
            bail!("dir with key '{}' not found", key)
        }
    }

    pub fn get_dir(&self, inode: Inode) -> Result<types::Dir> {
        if inode.ino() == 1 {
            return self.get_root();
        }

        let mut stmt = self
            .con
            .prepare("select value from entries where rowid = ?")?;
        //make sure we use the dir part only for the query
        stmt.bind(1, inode.dir().ino() as i64)?;

        if let sqlite::State::Row = stmt.next()? {
            let bytes: Vec<u8> = stmt.read(0)?;
            Ok(types::Dir::new(&self, &bytes, inode.dir())?)
        } else {
            bail!("dir with inode '{:?}' not found", inode)
        }
    }

    pub fn get_dir_by_loc(&self, loc: &str) -> Result<types::Dir> {
        self.get_dir_by_key(&Hash::new(loc).hex())
    }

    fn get_root(&self) -> Result<types::Dir> {
        self.get_dir_by_loc("")
    }
}
