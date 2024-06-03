#[macro_use]
extern crate log;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::io::Read;

use anyhow::{Context, Result};
use clap::{ArgAction, Args, Parser, Subcommand};

use rfs::cache;
use rfs::fungi;
use rfs::store::{self, Router, Stores};

mod fs;
/// mount flists
#[derive(Parser, Debug)]
#[clap(name ="rfs", author, version = env!("GIT_VERSION"), about, long_about = None)]
struct Options {
    /// enable debugging logs
    #[clap(long, action=ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// mount an FL
    Mount(MountOptions),
    /// create an FL and upload blocks to provided storage
    Pack(PackOptions),
    /// unpack (downloads) content of an FL the provided location
    Unpack(UnpackOptions),
}

#[derive(Args, Debug)]
struct MountOptions {
    /// path to metadata file (flist)
    #[clap(short, long)]
    meta: String,

    /// directory used as cache for downloaded file chuncks
    #[clap(short, long, default_value_t = String::from("/tmp/cache"))]
    cache: String,

    /// run in the background.
    #[clap(short, long)]
    daemon: bool,

    /// log file only used with daemon mode
    #[clap(short, long)]
    log: Option<String>,

    /// target mountpoint
    target: String,
}

#[derive(Args, Debug)]
struct PackOptions {
    /// path to metadata file (flist)
    #[clap(short, long)]
    meta: String,

    /// store url in the format [xx-xx=]<url>. the range xx-xx is optional and used for
    /// sharding. the URL is per store type, please check docs for more information
    #[clap(short, long, action=ArgAction::Append)]
    store: Vec<String>,

    /// no_strip_password disable automatic password stripping from store url, otherwise password will be stored in the fl.
    #[clap(long, default_value_t = false)]
    no_strip_password: bool,

    /// target directory to upload
    target: String,
}

#[derive(Args, Debug)]
struct UnpackOptions {
    /// path to metadata file (flist)
    #[clap(short, long)]
    meta: String,

    /// directory used as cache for downloaded file chuncks
    #[clap(short, long, default_value_t = String::from("/tmp/cache"))]
    cache: String,

    /// preserve files ownership from the FL, otherwise use the current user ownership
    /// setting this flag to true normally requires sudo
    #[clap(short, long, default_value_t = false)]
    preserve_ownership: bool,

    /// target directory to upload
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

    match opts.command {
        Commands::Mount(opts) => mount(opts),
        Commands::Pack(opts) => pack(opts),
        Commands::Unpack(opts) => unpack(opts),
    }
}

fn pack(opts: PackOptions) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async move {
        let store = store::parse_router(opts.store.as_slice()).await?;
        let meta = fungi::Writer::new(opts.meta).await?;
        rfs::pack(meta, store, opts.target, !opts.no_strip_password).await?;

        Ok(())
    })
}

fn unpack(opts: UnpackOptions) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async move {
        let meta = fungi::Reader::new(opts.meta)
            .await
            .context("failed to initialize metadata database")?;

        let router = get_router(&meta).await?;

        let cache = cache::Cache::new(opts.cache, router);
        rfs::unpack(&meta, &cache, opts.target, opts.preserve_ownership).await?;
        Ok(())
    })
}

fn mount(opts: MountOptions) -> Result<()> {
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
            daemonize::Outcome::Parent(result) => {
                result.context("daemonize")?;
                wait_child(target, pid_file);
                return Ok(());
            }
            _ => {}
        }
    }

    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(fuse(opts))
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
        error!("failed to read pid_file: {:#}", e);
    }
    let pid = buf.parse::<i32>();
    match pid {
        Err(e) => error!("failed to parse pid_file contents {}: {:#}", buf, e),
        Ok(v) => {
            let _ = signal::kill(Pid::from_raw(v), Signal::SIGTERM);
        } // probably the child exited on its own
    }
    // cleanup is not performed if the process is terminated with exit(2)
    drop(pid_file);
    eprintln!("failed to mount in under 5 seconds, please check logs for more information");
    std::process::exit(1);
}

async fn fuse(opts: MountOptions) -> Result<()> {
    let meta = fungi::Reader::new(opts.meta)
        .await
        .context("failed to initialize metadata database")?;

    let router = get_router(&meta).await?;

    let cache = cache::Cache::new(opts.cache, router);
    let filesystem = fs::Filesystem::new(meta, cache);

    filesystem.mount(opts.target).await
}

async fn get_router(meta: &fungi::Reader) -> Result<Router<Stores>> {
    let mut router = store::Router::new();

    for route in meta.routes().await.context("failed to get store routes")? {
        let store = store::make(&route.url)
            .await
            .with_context(|| format!("failed to initialize store '{}'", route.url))?;
        router.add(route.start, route.end, store);
    }

    Ok(router)
}
