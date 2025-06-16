#[macro_use]
extern crate log;
//use futures::future::ok;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::error::Error;
use std::io::Read;

use anyhow::{Context, Result};
use clap::{ArgAction, Args, Parser, Subcommand};

use rfs::fungi;
use rfs::store::{self};
use rfs::{
    cache, config, download, download_dir, exists, exists_by_hash, get_token_from_server,
    publish_website, sync, tree_visitor::TreeVisitor, upload, upload_dir,
};

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
    /// run the fl-server
    Server(ServerOptions),
    /// upload a file to a server
    Upload(UploadFileOptions),
    /// upload a directory to a server
    UploadDir(UploadDirOptions),
    /// check a file to a server, splitting it into blocks
    Exists(ExistsOptions),
    /// download a file from a server using its hash
    Download(DownloadOptions),
    /// download a directory from a server using its flist hash
    DownloadDir(DownloadDirOptions),
    /// create an flist from a directory
    FlistCreate(FlistCreateOptions),
    /// Publish a website
    WebsitePublish(WebsitePublishOptions),
    /// Sync files or blocks between two servers
    Sync(SyncOptions),
    /// retrieve a token using username and password
    Token(TokenOptions),
    /// flist inspection operations
    Flist(FlistOptions),
}

#[derive(Args, Debug)]
struct FlistOptions {
    #[command(subcommand)]
    command: FlistCommands,
}

#[derive(Subcommand, Debug)]
enum FlistCommands {
    /// show tree structure of an flist
    Tree(FlistInspectionOptions),
    /// inspect an flist by path or hash
    Inspect(FlistInspectionOptions),
}

#[derive(Args, Debug)]
struct FlistInspectionOptions {
    /// flist path or hash
    target: String,

    /// server URL for hash-based operations
    #[clap(long)]
    server_url: Option<String>,
}

#[derive(Args, Debug)]
struct SyncOptions {
    /// Hash of the file or block to sync
    #[clap(short, long)]
    hash: Option<String>,

    /// Source server URL (e.g., http://localhost:8080)
    #[clap(short, long, default_value_t = String::from("http://localhost:8080"))]
    source: String,

    /// Destination server URL (e.g., http://localhost:8081)
    #[clap(short, long, default_value_t = String::from("http://localhost:8081"))]
    destination: String,

    /// Block size for splitting files (only used if a file/directory is provided)
    #[clap(short, long, default_value_t = 1024 * 1024)] // 1MB
    block_size: usize,

    /// authentication token for the server
    #[clap(long, default_value_t = std::env::var("RFS_TOKEN").unwrap_or_default())]
    token: String,
}

#[derive(Args, Debug)]
struct WebsitePublishOptions {
    /// Path to the website directory
    path: String,

    /// Server URL (e.g., http://localhost:8080)
    #[clap(short, long, default_value_t = String::from("http://localhost:8080"))]
    server: String,

    /// Block size for splitting the files
    #[clap(short, long, default_value_t = 1024 * 1024)] // 1MB
    block_size: usize,

    /// authentication token for the server
    #[clap(long, default_value_t = std::env::var("RFS_TOKEN").unwrap_or_default())]
    token: String,
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

#[derive(Args, Debug)]
struct ServerOptions {
    /// config file path
    #[clap(short, long)]
    config_path: String,

    /// enable debugging logs
    #[clap(short, long, action=ArgAction::Count)]
    debug: u8,
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
struct UploadFileOptions {
    /// path to the file to upload
    path: String,

    /// server URL (e.g., http://localhost:8080)
    #[clap(short, long, default_value_t = String::from("http://localhost:8080"))]
    server: String,

    /// block size for splitting the file
    #[clap(short, long, default_value_t = 1024 * 1024)] // 1MB
    block_size: usize,

    /// authentication token for the server
    #[clap(long, default_value_t = std::env::var("RFS_TOKEN").unwrap_or_default())]
    token: String,
}

#[derive(Args, Debug)]
struct UploadDirOptions {
    /// path to the directory to upload
    path: String,

    /// server URL (e.g., http://localhost:8080)
    #[clap(short, long, default_value_t = String::from("http://localhost:8080"))]
    server: String,

    /// block size for splitting the files
    #[clap(short, long, default_value_t = 1024 * 1024)] // 1MB
    block_size: usize,

    /// create and upload flist file
    #[clap(long)]
    create_flist: bool,

    /// path to output the flist file
    #[clap(long)]
    flist_output: Option<String>,

    /// authentication token for the server
    #[clap(long, default_value_t = std::env::var("RFS_TOKEN").unwrap_or_default())]
    token: String,
}

#[derive(Args, Debug)]
struct DownloadOptions {
    /// hash of the file to download
    hash: String,

    /// name to save the downloaded file as
    #[clap(short, long)]
    output: String,

    /// server URL (e.g., http://localhost:8080)
    #[clap(short, long, default_value_t = String::from("http://localhost:8080"))]
    server: String,
}

#[derive(Args, Debug)]
struct DownloadDirOptions {
    /// hash of the flist to download
    hash: String,

    /// directory to save the downloaded files to
    #[clap(short, long)]
    output: String,

    /// server URL (e.g., http://localhost:8080)
    #[clap(short, long, default_value_t = String::from("http://localhost:8080"))]
    server: String,
}

#[derive(Args, Debug)]
struct ExistsOptions {
    /// path to the file or hash to check
    file_or_hash: String,

    /// server URL (e.g., http://localhost:8080)
    #[clap(short, long, default_value_t = String::from("http://localhost:8080"))]
    server: String,

    /// block size for splitting the file (only used if a file is provided)
    #[clap(short, long, default_value_t = 1024 * 1024)] // 1MB
    block_size: usize,
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

#[derive(Args, Debug)]
struct FlistCreateOptions {
    /// path to the directory to create the flist from
    directory: String,

    /// path to output the flist file
    #[clap(short, long)]
    output: String,

    /// server URL (e.g., http://localhost:8080)
    #[clap(short, long, default_value_t = String::from("http://localhost:8080"))]
    server: String,

    /// block size for splitting the files
    #[clap(short, long, default_value_t = 1024 * 1024)] // 1MB
    block_size: usize,

    /// authentication token for the server
    #[clap(long, default_value_t = std::env::var("RFS_TOKEN").unwrap_or_default())]
    token: String,
}

#[derive(Args, Debug)]
struct TokenOptions {
    /// username for authentication
    #[clap(short, long)]
    username: String,

    /// password for authentication
    #[clap(short, long)]
    password: String,

    /// server URL (e.g., http://localhost:8080)
    #[clap(short, long, default_value_t = String::from("http://localhost:8080"))]
    server: String,
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
        Commands::Server(opts) => server(opts),
        Commands::Upload(opts) => upload_file(opts),
        Commands::UploadDir(opts) => upload_directory(opts),
        Commands::Download(opts) => download_file(opts),
        Commands::DownloadDir(opts) => download_directory(opts),
        Commands::Exists(opts) => hash_or_file_exists(opts),
        Commands::FlistCreate(opts) => create_flist(opts),
        Commands::WebsitePublish(opts) => publish_website_command(opts),
        Commands::Sync(opts) => sync_command(opts),
        Commands::Token(opts) => get_token(opts),
        Commands::Flist(opts) => flist_command(opts),
    }
}

fn get_token(opts: TokenOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024)
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        let token = get_token_from_server(&opts.server, &opts.username, &opts.password)
            .await
            .context("Failed to retrieve token")?;
        println!("Token: {}", token);
        Ok(())
    })
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

fn flist_command(opts: FlistOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024)
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        match opts.command {
            FlistCommands::Tree(opts) => flist_tree(opts).await,
            FlistCommands::Inspect(opts) => flist_inspect(opts).await,
        }
    })
}

async fn flist_tree(opts: FlistInspectionOptions) -> Result<()> {
    if opts.server_url.is_some() {
        let server_url = opts.server_url.unwrap();
        let temp_flist = format!("/tmp/flist_{}.fl", &opts.target);

        download(&opts.target, &temp_flist, server_url)
            .await
            .context("Failed to download flist from server")?;

        let meta = fungi::Reader::new(&temp_flist)
            .await
            .context("failed to initialize metadata database from downloaded flist")?;

        let mut visitor = TreeVisitor::new();
        meta.walk(&mut visitor).await?;

        if let Err(e) = tokio::fs::remove_file(&temp_flist).await {
            warn!(
                "Failed to clean up temporary flist file {}: {}",
                temp_flist, e
            );
        }
    } else {
        let meta = fungi::Reader::new(&opts.target)
            .await
            .context("failed to initialize metadata database")?;

        let mut visitor = TreeVisitor::new();
        meta.walk(&mut visitor).await?;
    }

    Ok(())
}

async fn flist_inspect(opts: FlistInspectionOptions) -> Result<()> {
    if opts.server_url.is_some() {
        let server_url = opts.server_url.unwrap();
        let temp_flist = format!("/tmp/flist_{}.fl", &opts.target);

        download(&opts.target, &temp_flist, server_url)
            .await
            .context("Failed to download flist from server")?;

        let meta = fungi::Reader::new(&temp_flist)
            .await
            .context("failed to initialize metadata database from downloaded flist")?;

        let mut visitor = rfs::flist_inspector::InspectVisitor::new();
        meta.walk(&mut visitor).await?;

        if let Err(e) = tokio::fs::remove_file(&temp_flist).await {
            warn!(
                "Failed to clean up temporary flist file {}: {}",
                temp_flist, e
            );
        }
    } else {
        let meta = fungi::Reader::new(&opts.target)
            .await
            .context("failed to initialize metadata database")?;

        let mut visitor = rfs::flist_inspector::InspectVisitor::new();
        meta.walk(&mut visitor).await?;
        visitor.print_summary(&opts.target);
    }

    Ok(())
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

        if let daemonize::Outcome::Parent(result) = daemon.execute() {
            result.context("daemonize")?;
            wait_child(target, pid_file);
            return Ok(());
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

fn server(opts: ServerOptions) -> Result<()> {
    use std::process::{Command, Stdio};

    println!("Starting fl-server with config: {}", opts.config_path);

    // Find the fl-server binary in the same directory as the mycofs binary
    let current_exe = std::env::current_exe()?;
    let bin_dir = current_exe
        .parent()
        .context("Failed to get binary directory")?;
    let fl_server_path = bin_dir.join("fl-server");

    // Build the command with proper arguments
    let mut cmd = Command::new(fl_server_path);

    // Add config path
    cmd.arg("-c").arg(&opts.config_path);

    // Add debug flags if specified
    if opts.debug > 0 {
        for _ in 0..opts.debug {
            cmd.arg("-d");
        }
    }

    // Make sure we can see the output
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    // Run the fl-server binary
    let status = cmd.status().context("Failed to execute fl-server")?;

    if !status.success() {
        anyhow::bail!("fl-server exited with status: {}", status);
    }

    Ok(())
}

fn upload_file(opts: UploadFileOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        let path = std::path::Path::new(&opts.path);

        if !path.is_file() {
            return Err(anyhow::anyhow!("Not a valid file: {}", opts.path));
        }

        // Upload a single file
        upload(&opts.path, opts.server, Some(opts.block_size), &opts.token)
            .await
            .context("Failed to upload file")?;

        Ok(())
    })
}

fn upload_directory(opts: UploadDirOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(32 * 1024 * 1024) // Increased stack size to prevent overflow
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        let path = std::path::Path::new(&opts.path);

        if !path.is_dir() {
            return Err(anyhow::anyhow!("Not a valid directory: {}", opts.path));
        }

        // Upload a directory
        upload_dir(
            &opts.path,
            opts.server,
            Some(opts.block_size),
            &opts.token,
            opts.create_flist,
            opts.flist_output.as_deref(),
        )
        .await
        .context("Failed to upload directory")?;

        Ok(())
    })
}

fn create_flist(opts: FlistCreateOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        upload_dir(
            &opts.directory,
            opts.server,
            Some(opts.block_size),
            &opts.token,
            true,
            Some(&opts.output),
        )
        .await
        .context("Failed to upload directory")?;
        Ok(())
    })
}

fn hash_or_file_exists(opts: ExistsOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        if std::path::Path::new(&opts.file_or_hash).exists() {
            // If it's a file, check its existence by splitting into blocks
            exists(&opts.file_or_hash, opts.server, Some(opts.block_size))
                .await
                .context("Failed to check file")?;
        } else {
            // If it's a hash, directly check its existence on the server
            exists_by_hash(opts.file_or_hash, opts.server)
                .await
                .context("Failed to check hash")?;
        }
        Ok(())
    })
}

fn download_file(opts: DownloadOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        download(&opts.hash, &opts.output, opts.server)
            .await
            .context("Failed to download file")?;
        Ok(())
    })
}

fn download_directory(opts: DownloadDirOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024) // Use a larger stack size
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        download_dir(&opts.hash, &opts.output, opts.server)
            .await
            .context("Failed to download directory")?;
        Ok(())
    })
}

fn sync_command(opts: SyncOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024)
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        sync(
            opts.hash.as_deref(),
            &opts.source,
            &opts.destination,
            &opts.token,
        )
        .await
        .context("Failed to sync between servers")?;
        Ok(())
    })
}

fn publish_website_command(opts: WebsitePublishOptions) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024)
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        publish_website(&opts.path, opts.server, Some(opts.block_size), &opts.token)
            .await
            .context("Failed to publish website")?;
        Ok(())
    })
}
