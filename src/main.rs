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

struct Options {
    hub: String,
    meta: String,
    cache: String,
    target: String,
}

fn main() -> Result<()> {
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
            Arg::with_name("daemon")
                .short("d")
                .long("daemon")
                .help("daemonize process"),
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

    let opt = Options {
        hub: matches.value_of("hub").unwrap().into(),
        meta: matches.value_of("meta").unwrap().into(),
        cache: matches.value_of("cache").unwrap().into(),
        target: matches.value_of("target").unwrap().into(),
    };

    // if matches.is_present("daemon") {
    //     let out = std::fs::File::create("/tmp/fs.out.log")?;
    //     let err = out.try_clone()?;
    //     daemonize::Daemonize::new()
    //         .stdout(out)
    //         .stderr(err)
    //         .exit_action(|| println!("forked, should wait for mount"))
    //         .start()?;
    // }

    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(app(opt))
}

async fn app(opts: Options) -> Result<()> {
    let cache = cache::Cache::new(opts.hub, opts.cache).await?;
    let mgr = meta::Metadata::open(opts.meta).await?;
    let filesystem = fs::Filesystem::new(mgr, cache);

    filesystem.mount(opts.target).await
}
