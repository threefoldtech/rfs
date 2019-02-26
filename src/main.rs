extern crate fuse;
extern crate libc;
#[macro_use]
extern crate log;
extern crate capnp;
extern crate simple_logger;
extern crate sqlite;
extern crate blake2;

use std::ffi;
use std::path;

mod fs;
mod meta;

pub mod schema_capnp;

fn main() {
    // cae66941d9efbd404e4d88758ea67670

    let mgr = meta::Manager::new("/tmp/flistdb.sqlite3".to_string()).unwrap();
    mgr.get_root().unwrap();
    println!("got root");
    return;

    let con = sqlite::open("/tmp/flistdb.sqlite3").unwrap();

    let mut statement = con.prepare("select key, value from entries;").unwrap();
    while let sqlite::State::Row = statement.next().unwrap() {
        let key: String = statement.read(0).unwrap();
        let mut value: Vec<u8> = statement.read(1).unwrap();
        let mut slice: &[u8] = value.as_ref();

        //capnp::serialize::read_message();
        let msg = capnp::serialize::read_message(
            &mut slice,
            capnp::message::ReaderOptions::default()
        ).unwrap();

        let dir = msg.get_root::<schema_capnp::dir::Reader>().unwrap();

        // let msg = capnp::serialize::read_message(
        //     value.as_slice(),
        //     capnp::message::ReaderOptions::default(),
        // )
        // .unwrap();

        println!("Key {} Value {:?}/{:?}", key, dir.get_location(), dir.get_name());
    }
}

// fn main() {
//     simple_logger::init_with_level(log::Level::Debug).unwrap();
//     let p = path::Path::new("/tmp/mnt");

//     let o: [&ffi::OsStr; 0] = [];

//     let f = fs::Filesystem::new();

//     fuse::mount(f, &p, &o).unwrap();
// }
