#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate log;
use anyhow::Result;
use clap::{App, Arg};

mod cache;
mod fs;
mod meta;
pub mod schema_capnp {
    include!(concat!(env!("OUT_DIR"), "/schema_capnp.rs"));
}

/*
"-cache", f.cache,
"-meta", flistPath,
"-storage-url", storage,
"-daemon",
"-log", logPath,
// this is always read-only
"-ro",
*/
#[tokio::main]
async fn main() -> Result<()> {
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

    if matches.is_present("debug") {
        simple_logger::init_with_level(log::Level::Debug)?;
    } else {
        simple_logger::init_with_level(log::Level::Info)?;
    }

    let cache = cache::Cache::new(
        matches.value_of("hub").unwrap(),
        matches.value_of("cache").unwrap(),
    )
    .await?;

    let mgr = meta::Metadata::open(matches.value_of("meta").unwrap()).await?;

    let filesystem = fs::Filesystem::new(mgr, cache);

    filesystem.mount(matches.value_of("target").unwrap()).await
}
