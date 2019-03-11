extern crate fuse;
extern crate libc;
#[macro_use]
extern crate log;
extern crate blake2;
extern crate capnp;
extern crate crossbeam;
extern crate fs2;
extern crate lru;
extern crate redis;
extern crate simple_logger;
extern crate sqlite;
extern crate time;

use std::ffi;
use std::path;

mod fs;
mod meta;

pub mod schema_capnp;

fn main() {
    let mgr = meta::Manager::new("/tmp/flistdb.sqlite3".to_string()).unwrap();
    //let root = mgr.get_root().unwrap();
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let p = path::Path::new("/tmp/mnt");

    let o: [&ffi::OsStr; 0] = [];

    let f = fs::Filesystem::new(&mgr);

    fuse::mount(f, &p, &o).unwrap();
}
