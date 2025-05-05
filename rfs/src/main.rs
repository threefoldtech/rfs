#[macro_use]
extern crate log;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::error::Error;
use std::io::Read;

use anyhow::{Context, Result};
use clap::{ArgAction, Args, Parser, Subcommand};

use rfs::fungi;
use rfs::store::{self};
use rfs::{cache, config};

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
    /// clone copies the data from the stores of an FL to another stores
    Clone(CloneOptions),
    /// list or modify FL metadata and stores
    Config(ConfigOptions),
    /// merge 2 or more FLs into a new one
    Merge(MergeOptions),
    /// convert a docker image to an FL
    Docker(DockerOptions),
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

    /// target directory for unpacking
    target: String,
}

#[derive(Args, Debug)]
struct CloneOptions {
    /// path to metadata file (flist)
    #[clap(short, long)]
    meta: String,

    /// store url in the format [xx-xx=]<url>. the range xx-xx is optional and used for
    /// sharding. the URL is per store type, please check docs for more information
    #[clap(short, long, action=ArgAction::Append)]
    store: Vec<String>,

    /// directory used as cache for downloaded file chunks
    #[clap(short, long, default_value_t = String::from("/tmp/cache"))]
    cache: String,
}

#[derive(Args, Debug)]
struct MergeOptions {
    /// path to metadata file (flist)
    meta: String,

    #[clap(short, long, action=ArgAction::Append, required = true)]
    store: Vec<String>,

    #[clap(long, default_value_t = false)]
    no_strip_password: bool,

    #[clap(action=ArgAction::Append, required = true)]
    target_flists: Vec<String>,

    #[clap(short, long, default_value_t = String::from("/tmp/cache"))]
    cache: String,
}

impl MergeOptions {
    fn validate(&self) -> Result<()> {
        if self.target_flists.len() < 2 {
            return Err(anyhow::anyhow!(
                "At least 2 target file lists are required for merge operation"
            ));
        }
        Ok(())
    }
}

#[derive(Args, Debug)]
struct ConfigOptions {
    /// path to metadata file (flist)
    #[clap(short, long)]
    meta: String,

    #[command(subcommand)]
    command: ConfigCommands,
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    #[command(subcommand)]
    Tag(TagOperation),
    #[command(subcommand)]
    Store(StoreOperation),
}

#[derive(Subcommand, Debug)]
enum TagOperation {
    List,
    Add(TagAddOptions),
    Delete(TagDeleteOptions),
}

#[derive(Args, Debug)]
struct TagAddOptions {
    /// pair of key-values separated with '='
    #[clap(short, long, value_parser = parse_key_val::<String, String>, number_of_values = 1)]
    tag: Vec<(String, String)>,
}

#[derive(Args, Debug)]
struct TagDeleteOptions {
    /// key to remove
    #[clap(short, long, action=ArgAction::Append)]
    key: Vec<String>,
    /// remove all tags
    #[clap(short, long, default_value_t = false)]
    all: bool,
}

#[derive(Subcommand, Debug)]
enum StoreOperation {
    List,
    Add(StoreAddOptions),
    Delete(StoreDeleteOptions),
}

#[derive(Args, Debug)]
struct StoreAddOptions {
    /// store url in the format [xx-xx=]<url>. the range xx-xx is optional and used for
    /// sharding. the URL is per store type, please check docs for more information
    #[clap(short, long, action=ArgAction::Append)]
    store: Vec<String>,
}

#[derive(Args, Debug)]
struct StoreDeleteOptions {
    /// store to remove
    #[clap(short, long, action=ArgAction::Append)]
    store: Vec<String>,
    /// remove all stores
    #[clap(short, long, default_value_t = false)]
    all: bool,
}

#[derive(Args, Debug)]
struct DockerOptions {
    /// name of the docker image to be converted to flist
    #[clap(short, long, required = true)]
    image_name: String,

    /// store url for rfs in the format [xx-xx=]<url>. the range xx-xx is optional and used for
    /// sharding. the URL is per store type, please check docs for more information
    #[clap(short, long, required = true, action=ArgAction::Append)]
    store: Vec<String>,

    // docker credentials
    /// docker hub server username
    #[clap(long, required = false)]
    username: Option<String>,

    /// docker hub server password
    #[clap(long, required = false)]
    password: Option<String>,

    /// docker hub server auth
    #[clap(long, required = false)]
    auth: Option<String>,

    /// docker hub server email
    #[clap(long, required = false)]
    email: Option<String>,

    /// docker hub server address
    #[clap(long, required = false)]
    server_address: Option<String>,

    /// docker hub server identity token
    #[clap(long, required = false)]
    identity_token: Option<String>,

    /// docker hub server registry token
    #[clap(long, required = false)]
    registry_token: Option<String>,
}

/// Parse a single key-value pair
fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
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
        Commands::Clone(opts) => clone(opts),
        Commands::Config(opts) => config(opts),
        Commands::Merge(opts) => merge(opts),
        Commands::Docker(opts) => docker(opts),
    }
}

fn pack(opts: PackOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        let store = store::parse_router(opts.store.as_slice()).await?;
        let meta = fungi::Writer::new(opts.meta, true).await?;
        rfs::pack(meta, store, opts.target, !opts.no_strip_password, None).await?;

        Ok(())
    })
}

fn unpack(opts: UnpackOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        let meta = fungi::Reader::new(opts.meta)
            .await
            .context("failed to initialize metadata database")?;

        let router = store::get_router(&meta).await?;

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

    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

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

    let router = store::get_router(&meta).await?;

    let cache = cache::Cache::new(opts.cache, router);
    let filesystem = fs::Filesystem::new(meta, cache);

    filesystem.mount(opts.target).await
}

fn clone(opts: CloneOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        let store = store::parse_router(opts.store.as_slice()).await?;
        let meta = fungi::Reader::new(opts.meta)
            .await
            .context("failed to initialize metadata database")?;

        let router = store::get_router(&meta).await?;

        let cache = cache::Cache::new(opts.cache, router);
        rfs::clone(meta, store, cache).await?;

        Ok(())
    })
}
fn config(opts: ConfigOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        let writer = fungi::Writer::new(opts.meta.clone(), false)
            .await
            .context("failed to initialize metadata database")?;

        let reader = fungi::Reader::new(opts.meta)
            .await
            .context("failed to initialize metadata database")?;

        match opts.command {
            ConfigCommands::Tag(opts) => match opts {
                TagOperation::List => config::tag_list(reader).await?,
                TagOperation::Add(opts) => config::tag_add(writer, opts.tag).await?,
                TagOperation::Delete(opts) => {
                    config::tag_delete(writer, opts.key, opts.all).await?
                }
            },
            ConfigCommands::Store(opts) => match opts {
                StoreOperation::List => config::store_list(reader).await?,
                StoreOperation::Add(opts) => config::store_add(writer, opts.store).await?,
                StoreOperation::Delete(opts) => {
                    config::store_delete(writer, opts.store, opts.all).await?
                }
            },
        }

        Ok(())
    })
}

fn merge(opts: MergeOptions) -> Result<()> {
    opts.validate()?;

    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async move {
        let store = store::parse_router(opts.store.as_slice()).await?;
        let meta = fungi::Writer::new(opts.meta, true).await?;
        rfs::merge(
            meta,
            store,
            !opts.no_strip_password,
            opts.target_flists,
            opts.cache,
        )
        .await?;
        Ok(())
    })
}

fn docker(opts: DockerOptions) -> Result<()> {
    use bollard::auth::DockerCredentials;
    use uuid::Uuid;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        let mut docker_image = opts.image_name.to_string();
        if !docker_image.contains(':') {
            docker_image.push_str(":latest");
        }

        let credentials = Some(DockerCredentials {
            username: opts.username,
            password: opts.password,
            auth: opts.auth,
            email: opts.email,
            serveraddress: opts.server_address,
            identitytoken: opts.identity_token,
            registrytoken: opts.registry_token,
        });

        let fl_name = docker_image.replace([':', '/'], "-") + ".fl";
        let meta = fungi::Writer::new(&fl_name, true).await?;
        let store = store::parse_router(&opts.store).await?;

        let container_name = Uuid::new_v4().to_string();
        let docker_tmp_dir =
            tempdir::TempDir::new(&container_name).expect("failed to create tmp directory");

        let mut docker_to_fl =
            rfs::DockerImageToFlist::new(meta, docker_image, credentials, docker_tmp_dir);
        let res = docker_to_fl.convert(store, None).await;

        // remove the file created with the writer if fl creation failed
        if res.is_err() {
            tokio::fs::remove_file(fl_name).await?;
            return res;
        }

        Ok(())
    })
}
