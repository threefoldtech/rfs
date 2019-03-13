use crate::fs;
use crate::meta;
use clap;
use log;
use std::error::Error;
use std::ffi::OsStr;
use std::path;

type Result<T> = std::result::Result<T, Box<Error>>;

pub fn run(matches: &clap::ArgMatches) -> Result<()> {
    let meta_dir = matches.value_of("meta").unwrap(); //it is required
    let target = matches.value_of("target").unwrap(); //it is required
    let hub = matches
        .value_of("hub")
        .unwrap_or("redis://hub.grid.tf:9900");
    let cache = matches.value_of("cache").unwrap_or("/tmp/cache");

    let mut level = log::Level::Info;
    if matches.is_present("debug") {
        level = log::Level::Debug;
    }

    simple_logger::init_with_level(level)?;
    let db = path::Path::new(meta_dir).join("flistdb.sqlite3");
    let mgr = meta::Manager::new(db)?;

    let o: [&OsStr; 1] = [&OsStr::new("ro")];

    let f = fs::Filesystem::new(&mgr, hub, cache).unwrap();

    fuse::mount(f, &target, &o)?;

    Ok(())
}
