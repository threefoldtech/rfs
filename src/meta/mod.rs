use blake2::VarBlake2b;
use blake2::digest::{Input, VariableOutput};
use sqlite::{open, Statement};
use capnp::{serialize, message};
use std::error;
use std::fmt;
use crate::schema_capnp::dir;

struct Hash(Vec<u8>);

impl Hash {
    fn new(w: &str) -> Hash {
        let mut hasher = VarBlake2b::new(16).unwrap();
        hasher.input(w);

        Hash(hasher.vec_result())
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
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
    details: String
}

impl Error {
    fn new(msg: String) -> Error {
        Error{details: msg}
    }

    fn boxed(msg: String) -> Box<Error> {
        Box::new(Self::new(msg))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.details)
    }
}

impl error::Error for Error {}

pub struct Manager {
    con: sqlite::Connection,
}

impl Manager {
    pub fn new(path: String) -> Result<Manager, Box<error::Error>> {
        let con = sqlite::open(path)?;

        Ok(Manager{
            con: con,
        })
    }

    fn dir_from(bytes: &mut Vec<u8>) -> Result<(), Box<error::Error>> {
        let mut raw: &[u8] = bytes.as_ref();

        let msg = serialize::read_message(&mut raw, message::ReaderOptions::default())?;
        let dir = msg.get_root::<dir::Reader>()?;

        println!("Root {:?}", dir.get_location());
        Ok(())
    }

    pub fn get_dir(&self, loc: &str) -> Result<(), Box<error::Error>>{
        let mut stmt = self.con.prepare("select value from entries where key = ?")?;
        stmt.bind(1, Hash::new(loc))?;

        if let sqlite::State::Row = stmt.next()? {
            let mut bytes: Vec<u8> = stmt.read(0)?;
            let dir = Self::dir_from(&mut bytes)?;
            //dir.get_name();
            Ok(())
        } else {
            Err(Error::boxed(format!("{} not found", loc)))
        }
    }

    pub fn get_root(&self) -> Result<(), Box<error::Error>>{
        self.get_dir("")
    }
}