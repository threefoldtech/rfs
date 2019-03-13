extern crate fuse;
extern crate libc;
#[macro_use]
extern crate log;
extern crate blake2;
extern crate capnp;
extern crate clap;
extern crate crossbeam;
extern crate fs2;
extern crate lru;
extern crate redis;
extern crate simple_logger;
extern crate snappy;
extern crate sqlite;
extern crate time;
extern crate xxtea;

use clap::{App, Arg};
use std::ffi;
use std::path;

mod fs;
mod meta;
pub mod schema_capnp;

fn main() {
    let matches = App::new("mount flists")
        .version("0.1")
        .author("Muhamad Azmy")
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .help("enable debug logging"),
        )
        .arg(
            Arg::with_name("meta")
                .long("meta")
                .required(true)
                .takes_value(true)
                .value_name("META")
                .help("meta directory that has a .sqlite file from the flist"),
        )
        .arg(
            Arg::with_name("mount")
                .required(true)
                .value_name("MOUNT")
                .index(1),
        )
        .get_matches();

    // matches.get_matches();

    let meta = matches.value_of("meta").unwrap_or("default meta");
    println!("meta is {}", meta);
}

fn main2() {
    let mgr = meta::Manager::new("/tmp/flistdb.sqlite3".to_string()).unwrap();
    //let root = mgr.get_root().unwrap();
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let p = path::Path::new("/tmp/mnt");

    let o: [&ffi::OsStr; 0] = [];

    let f = fs::Filesystem::new(&mgr, "redis://hub.grid.tf:9900").unwrap();

    fuse::mount(f, &p, &o).unwrap();
}
