use crate::schema_capnp::dir;
use blake2::digest::{Input, VariableOutput};
use blake2::VarBlake2b;
use capnp::{message, serialize};
use sqlite::{open, Statement};
use std::error;
use std::fmt;

mod inode;
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
    mask: inode::Mask,
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

    fn get_inode_mask(con: &sqlite::Connection) -> Result<inode::Mask> {
        let mut stmt = con.prepare("select max(rowid) from entries")?;
        if let sqlite::State::Row = stmt.next()? {
            let max: i64 = stmt.read(0)?;
            Ok(inode::Mask::from(max as u64))
        } else {
            Err(Error::boxed("failed to get inode count".to_string()))
        }
    }

    fn dir_from(inode: u64, bytes: &mut Vec<u8>) -> Result<types::Dir> {
        let mut raw: &[u8] = bytes.as_ref();

        let msg = serialize::read_message(&mut raw, message::ReaderOptions::default())?;

        let dir = msg.get_root::<dir::Reader>()?;

        //println!("Root {:?}", dir.get_location());
        Ok(types::Dir::from(inode, &dir)?)
    }

    pub fn get_dir_by_key(&self, key: &str) -> Result<types::Dir> {
        let mut stmt = self
            .con
            .prepare("select rowid, value from entries where key = ?")?;
        stmt.bind(1, key)?;

        if let sqlite::State::Row = stmt.next()? {
            let inode: u64 = stmt.read::<i64>(0)? as u64;
            let mut bytes: Vec<u8> = stmt.read(1)?;
            Self::dir_from(inode, &mut bytes)
        } else {
            Err(Error::boxed(format!("dir with key '{}' not found", key)))
        }
    }

    pub fn get_dir(&self, inode: u64) -> Result<types::Dir> {
        let mut stmt = self
            .con
            .prepare("select value from entries where rowid = ?")?;
        stmt.bind(1, inode as i64)?;

        if let sqlite::State::Row = stmt.next()? {
            let mut bytes: Vec<u8> = stmt.read(0)?;
            Self::dir_from(inode, &mut bytes)
        } else {
            Err(Error::boxed(format!(
                "dir with inode '{}' not found",
                inode
            )))
        }
    }

    pub fn get_dir_by_loc(&self, loc: &str) -> Result<types::Dir> {
        self.get_dir_by_key(format!("{}", Hash::new(loc)).as_str())
    }

    pub fn get_root(&self) -> Result<types::Dir> {
        self.get_dir_by_loc("")
    }
}
