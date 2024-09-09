use anyhow::Result;
use bollard::auth::DockerCredentials;
use clap::{ArgAction, Parser};
use rfs::fungi;
use rfs::store::parse_router;

mod docker2fl;

#[derive(Parser, Debug)]
#[clap(name ="docker2fl", author, version = env!("GIT_VERSION"), about, long_about = None)]
struct Options {
    /// enable debugging logs
    #[clap(short, long, action=ArgAction::Count)]
    debug: u8,

    /// store url for rfs in the format [xx-xx=]<url>. the range xx-xx is optional and used for
    /// sharding. the URL is per store type, please check docs for more information
    #[clap(short, long, required = true, action=ArgAction::Append)]
    store: Vec<String>,

    /// name of the docker image to be converted to flist
    #[clap(short, long, required = true)]
    image_name: String,

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

#[tokio::main]
async fn main() -> Result<()> {
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
    let store = parse_router(&opts.store).await?;

    let res = docker2fl::convert(meta, store, &docker_image, credentials).await;

    // remove the file created with the writer if fl creation failed
    if res.is_err() {
        tokio::fs::remove_file(fl_name).await?;
        return res;
    }

    Ok(())
}
