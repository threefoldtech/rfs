#[macro_use]
extern crate log;
use cache::{ConnectionInfo, IntoConnectionInfo};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::io::Read;

use anyhow::{Context, Result};
use clap::{ArgAction, Parser};

mod cache;
mod fs;
mod fungi;
pub mod schema_capnp {
    include!(concat!(env!("OUT_DIR"), "/schema_capnp.rs"));
}

/// mount flists
#[derive(Parser, Debug)]
#[clap(name ="rfs", author, version = env!("GIT_VERSION"), about, long_about = None)]
struct Options {
    /// storage-url is a url to the backend where the data is stored
    /// this is a backup url to fall back to for backward compatability.
    /// In newered flists, this information is already included int he flist
    #[clap(
        short,
        long = "storage-url",
        default_value_t = String::from("redis://hub.grid.tf:9900")
    )]
    storage_url: String,
    /// path to metadata file (flist)
    #[clap(short, long)]
    meta: String,

    /// directory used as cache for downloaded file chuncks
    #[clap(short, long, default_value_t = String::from("/tmp/cache"))]
    cache: String,

    #[clap(short, long)]
    daemon: bool,

    /// enable debugging logs
    #[clap(long, action=ArgAction::Count)]
    debug: u8,

    /// log file only used with daemon mode
    #[clap(short, long)]
    log: Option<String>,

    /// hidden value
    #[clap(long = "ro", hide = true)]
    ro: bool,

    /// target mountpoint
    target: String,
}

fn main() -> Result<()> {
    let opts = Options::parse();

    simple_logger::SimpleLogger::new()
        .with_utc_timestamps()
        .with_level({
            match opts.debug {
                0 => log::LevelFilter::Info,
                1 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            }
        })
        .with_module_level("sqlx", log::Level::Error.to_level_filter())
        .init()?;

    log::debug!("options: {:#?}", opts);

    if is_mountpoint(&opts.target)? {
        eprintln!("target {} is already a mount point", opts.target);
        std::process::exit(1);
    }

    if opts.daemon {
        let pid_file = tempfile::NamedTempFile::new()?;
        let target = opts.target.clone();
        let mut daemon = daemonize::Daemonize::new()
            .working_directory(std::env::current_dir()?)
            .pid_file(pid_file.path());
        if let Some(ref log) = opts.log {
            let out = std::fs::File::create(log)?;
            let err = out.try_clone()?;
            daemon = daemon.stdout(out).stderr(err);
        }

        match daemon.execute() {
            daemonize::Outcome::Parent(Ok(_)) => {
                wait_child(target, pid_file);
                return Ok(());
            }
            daemonize::Outcome::Parent(Err(err)) => anyhow::bail!("failed to daemonize: {}", err),
            _ => {}
        }
    }

    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(app(opts))
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
    let mgr = fungi::Reader::new(opts.meta)
        .await
        .context("failed to initialize metadata database")?;

    // TODO: backend support with routers
    let cache_info: ConnectionInfo = opts.storage_url.into_connection_info()?;

    info!("backend: {}", cache_info);
    let cache = cache::Cache::new(cache_info, opts.cache)
        .await
        .context("failed to initialize cache")?;

    //TODO: print tags

    // match mgr.tags().await {
    //     Ok(tags) => {
    //         debug!("flist has {} tags", tags.len());
    //         for (k, v) in tags.iter() {
    //             debug!("[tag][{}]: {}", k, v);
    //         }
    //     }
    //     Err(err) => {
    //         error!("failed to extract flist tags: {}", err);
    //     }
    // }

    let filesystem = fs::Filesystem::new(mgr, cache);
    filesystem.mount(opts.target).await
}
