#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate log;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::io::Read;

use anyhow::{Context, Result};
use clap::{App, Arg};

mod cache;
mod fs;
mod meta;
pub mod schema_capnp {
    include!(concat!(env!("OUT_DIR"), "/schema_capnp.rs"));
}

const GIT_VERSION: &str =
    git_version::git_version!(args = ["--tags", "--always", "--dirty=-modified"]);

struct Options {
    hub: String,
    meta: String,
    cache: String,
    target: String,
    daemon: bool,
}

fn main() -> Result<()> {
    let matches = App::new("Mount flists")
        .version(GIT_VERSION)
        .author("Threefold Tech")
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
                .help("metadata file, can be a .flist file, a .sqlite3 file or a directory with a `flistdb.sqlite3` inside"),
        )
        .arg(
            Arg::with_name("hub")
                .long("storage-url")
                .help("storage url to retrieve files from. Url can end with /<namespace> to switch to a different namespace")
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
            Arg::with_name("log")
                .long("log")
                .takes_value(true)
                .help("log file only in daemon mode"),
        )
        .arg(
            Arg::with_name("ro")
            .long("ro")
            .hidden(true)
            .help("only for compatibility with command line interface of g8ufs")
        )
        .arg(
            Arg::with_name("target")
                .required(true)
                .value_name("TARGET")
                .index(1),
        )
        .get_matches();

    let mut logger = simple_logger::SimpleLogger::new()
        .with_utc_timestamps()
        .with_level(log::Level::Info.to_level_filter())
        .with_module_level("sqlx", log::Level::Error.to_level_filter());

    if matches.is_present("debug") {
        logger = logger.with_level(log::Level::Debug.to_level_filter())
    }

    logger.init()?;

    let opt = Options {
        hub: matches.value_of("hub").unwrap().into(),
        meta: matches.value_of("meta").unwrap().into(),
        cache: matches.value_of("cache").unwrap().into(),
        target: matches.value_of("target").unwrap().into(),
        daemon: matches.is_present("daemon"),
    };

    if is_mountpoint(&opt.target)? {
        eprintln!("target {} is already a mount point", opt.target);
        std::process::exit(1);
    }

    if opt.daemon {
        let pid_file = tempfile::NamedTempFile::new()?;
        let target = opt.target.clone();
        let mut daemon = daemonize::Daemonize::new()
            .working_directory(std::env::current_dir()?)
            .pid_file(pid_file.path());
        if matches.is_present("log") {
            let out = std::fs::File::create(matches.value_of("log").unwrap())?;
            let err = out.try_clone()?;
            daemon = daemon.stdout(out).stderr(err);
        }

        match daemon.execute() {
            daemonize::Outcome::Parent(Ok(_)) => {
                wait_child(target, pid_file);
                return Ok(());
            }
            daemonize::Outcome::Parent(Err(err)) => bail!("failed to daemonize: {}", err),
            _ => {}
        }
    }

    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(app(opt))
}

fn is_mountpoint<S: AsRef<str>>(target: S) -> Result<bool> {
    use std::process::Command;

    let output = Command::new("mountpoint")
        .arg("-q")
        .arg(target.as_ref())
        .output()
        .context("failed to check mountpoint")?;

    Ok(output.status.success())
}

fn wait_child(target: String, mut pid_file: tempfile::NamedTempFile) {
    for _ in 0..5 {
        if is_mountpoint(&target).unwrap() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    let mut buf = String::new();
    if let Err(e) = pid_file.read_to_string(&mut buf) {
        error!("failed to read pid_file: {}", e);
    }
    let pid = buf.parse::<i32>();
    match pid {
        Err(e) => error!("failed to parse pid_file contents {}: {}", buf, e),
        Ok(v) => {
            let _ = signal::kill(Pid::from_raw(v), Signal::SIGTERM);
        } // probably the child exited on its own
    }
    // cleanup is not performed if the process is terminated with exit(2)
    drop(pid_file);
    eprintln!("failed to mount in under 5 seconds, please check logs for more information");
    std::process::exit(1);
}

async fn app(opts: Options) -> Result<()> {
    let cache = cache::Cache::new(opts.hub, opts.cache)
        .await
        .context("failed to initialize cache")?;
    let mgr = meta::Metadata::open(opts.meta)
        .await
        .context("failed to initialize metadata database")?;

    //print tags
    match mgr.tags().await {
        Ok(tags) => {
            debug!("flist has {} tags", tags.len());
            for (k, v) in tags.iter() {
                info!("[tag][{}]: {}", k, v);
            }
        }
        Err(err) => {
            error!("failed to extract flist tags: {}", err);
        }
    }

    let filesystem = fs::Filesystem::new(mgr, cache);
    filesystem.mount(opts.target).await
}
