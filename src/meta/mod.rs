use crate::schema_capnp::dir;
use blake2::digest::{Input, VariableOutput};
use blake2::VarBlake2b;
use capnp::{message, serialize};
use sqlite::{open, Statement};
use std::error;
use std::fmt;

pub mod inode;
pub use inode::{Inode, Mask};

pub mod types;
pub use types::EntryKind;

pub type Result<T> = std::result::Result<T, Box<error::Error>>;

struct Hash(Vec<u8>);
impl Hash {
    fn new(w: &str) -> Hash {
        let mut hasher = VarBlake2b::new(16).unwrap();
        hasher.input(w);

        Hash(hasher.vec_result())
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        for i in self.0.as_slice() {
            write!(f, "{:02x}", i)?;
        }

        Ok(())
    }
}

impl sqlite::Bindable for Hash {
    fn bind(self, stmt: &mut Statement, i: usize) -> sqlite::Result<()> {
        stmt.bind(i, format!("{}", self).as_str())
    }
}

#[derive(Debug)]
struct Error {
    details: String,
}

impl Error {
    fn new(msg: String) -> Error {
        Error { details: msg }
    }

    fn boxed(msg: String) -> Box<Error> {
        Box::new(Self::new(msg))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl error::Error for Error {}

pub struct Manager {
    con: sqlite::Connection,
    mask: Mask,
}

impl Manager {
    pub fn new(path: String) -> Result<Manager> {
        let con = sqlite::open(path)?;
        let mask = Self::get_inode_mask(&con)?;

        Ok(Manager {
            con: con,
            mask: mask,
        })
    }

    pub fn get_inode(&self, ino: u64) -> Inode {
        Inode::new(self.mask, ino)
    }

    fn get_inode_mask(con: &sqlite::Connection) -> Result<inode::Mask> {
        let mut stmt = con.prepare("select max(rowid) from entries")?;
        if let sqlite::State::Row = stmt.next()? {
            let max: i64 = stmt.read(0)?;
            Ok(inode::Mask::from(max as u64))
        } else {
            Err(Error::boxed("failed to get inode count".to_string()))
        }
    }

    fn dir_inode_from_key(&self, key: &str) -> Option<Inode> {
        let mut stmt = self
            .con
            .prepare("select rowid from entries where key = ?")
            .ok()?;

        stmt.bind(1, key);
        if let sqlite::State::Row = stmt.next().ok()? {
            let id: u64 = stmt.read::<i64>(0).ok()? as u64;
            Some(Inode::new(self.mask, id))
        } else {
            None
        }
    }

    pub fn get_dir_by_key(&self, key: &str) -> Result<types::Dir> {
        let mut stmt = self
            .con
            .prepare("select rowid, value from entries where key = ?")?;
        stmt.bind(1, key)?;

        if let sqlite::State::Row = stmt.next()? {
            let id: u64 = stmt.read::<i64>(0)? as u64;
            let mut bytes: Vec<u8> = stmt.read(1)?;
            Ok(types::Dir::new(Inode::new(self.mask, id), bytes)?)
        } else {
            Err(Error::boxed(format!("dir with key '{}' not found", key)))
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
            let mut bytes: Vec<u8> = stmt.read(0)?;
            Ok(types::Dir::new(inode.dir(), bytes)?)
        } else {
            Err(Error::boxed(format!(
                "dir with inode '{:?}' not found",
                inode
            )))
        }
    }

    pub fn get_node<'a>(&'a self, inode: Inode) -> Result<Box<types::Node + 'a>> {
        let dir = Box::new(self.get_dir(inode.dir())?);
        let index = inode.index();
        let entries = dir.entries(self)?;
        if index == 0 {
            Ok(dir)
        } else if index <= entries.len() as u64 {
            let entry = &entries[(index - 1) as usize];
            Ok(Box::new(entry.clone()))
        } else {
            Err(Error::boxed(format!("entry {} not found", inode)))
        }
    }

    pub fn get_dir_by_loc(&self, loc: &str) -> Result<types::Dir> {
        self.get_dir_by_key(format!("{}", Hash::new(loc)).as_str())
    }

    fn get_root(&self) -> Result<types::Dir> {
        self.get_dir_by_loc("")
    }
}
