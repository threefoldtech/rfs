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
extern crate num_cpus;
extern crate redis;
extern crate simple_logger;
extern crate snappy;
extern crate sqlite;
extern crate time;
extern crate xxtea;

use clap::{App, Arg};

mod app;
mod fs;
mod meta;
mod schema_capnp;

fn main() {
    let matches = App::new("Mount Flists")
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
            Arg::with_name("hub")
                .long("storage-url")
                .help("storage url to retrieve files from")
                .default_value("redis://hub.grid.tf:9900"),
        )
        .arg(
            Arg::with_name("cache")
                .long("cache")
                .help("cache directory")
                .default_value("/tmp/cache"),
        )
        .arg(
            Arg::with_name("target")
                .required(true)
                .value_name("TARGET")
                .index(1),
        )
        .get_matches();

    match app::run(&matches) {
        Err(err) => eprintln!("{}", err),
        _ => {
            return;
        }
    };
}
