extern crate fuse;
extern crate libc;
#[macro_use]
extern crate log;
extern crate capnp;
extern crate simple_logger;

use std::ffi;
use std::path;

mod fs;
pub mod schema_capnp;

fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let p = path::Path::new("/tmp/mnt");

    let o: [&ffi::OsStr; 0] = [];

    let f = fs::Filesystem::new();

    fuse::mount(f, &p, &o).unwrap();
}
