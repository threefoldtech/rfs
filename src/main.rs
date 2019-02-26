extern crate fuse;
extern crate libc;
#[macro_use]
extern crate log;
extern crate capnp;
extern crate simple_logger;
extern crate sqlite;

use std::ffi;
use std::path;

mod fs;
pub mod schema_capnp;

fn main() {
    let con = sqlite::open("/tmp/flistdb.sqlite3").unwrap();
    let mut statement = con.prepare("select key, value from entries;").unwrap();
    while let sqlite::State::Row = statement.next().unwrap() {
        let key: String = statement.read(0).unwrap();
        let mut value: Vec<u8> = statement.read(1).unwrap();
        
        capnp::serialize::read_message(&mut value, capnp::message::ReaderOptions::default())
        let msg = capnp::serialize_packed::read_message(
            &mut value,
            capnp::message::ReaderOptions::default(),
        )
        .unwrap();

        println!("Key {} Value {}", key, value.len());
    }
}

// fn main() {
//     simple_logger::init_with_level(log::Level::Debug).unwrap();
//     let p = path::Path::new("/tmp/mnt");

//     let o: [&ffi::OsStr; 0] = [];

//     let f = fs::Filesystem::new();

//     fuse::mount(f, &p, &o).unwrap();
// }
