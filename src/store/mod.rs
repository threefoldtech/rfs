mod bs;
pub mod dir;
mod router;
pub mod zdb;
pub mod s3store;

use rand::seq::SliceRandom;
use std::{collections::HashMap, pin::Pin};

pub use bs::BlockStore;
use futures::Future;
use s3;

lazy_static::lazy_static! {
    static ref STORES: HashMap<String, Factory> = register_stores();
}

/// register_stores is used to register the stores built in types
/// so they can be created with a url
fn register_stores() -> HashMap<String, Factory> {
    let mut m: HashMap<String, Factory> = HashMap::default();
    m.insert("dir".into(), dir::make);
    m.insert("zdb".into(), zdb::make);
    m.insert("s3".into(), s3store::make);

    m
}

pub async fn make<U: AsRef<str>>(u: U) -> Result<Box<dyn Store>> {
    let parsed = url::Url::parse(u.as_ref())?;
    let factory = match STORES.get(parsed.scheme()) {
        None => return Err(Error::UnknownStore(parsed.scheme().into())),
        Some(factory) => factory,
    };

    factory(u.as_ref()).await
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("key not found")]
    KeyNotFound,
    #[error("invalid key")]
    InvalidKey,
    #[error("invalid blob")]
    InvalidBlob,
    #[error("key is not routable")]
    KeyNotRoutable,
    #[error("store is not available")]
    Unavailable,

    #[error("compression error: {0}")]
    Compression(#[from] snap::Error),

    #[error("encryption error")]
    EncryptionError,

    #[error("bucket creation error")]
    BucketCreationError,

    #[error("invalid host")]
    InvalidHost,
    #[error("invalid url configuration: {0}")]
    InvalidConfigs(String),
    // TODO: better display for the Box<Vec<Self>>
    #[error("multiple error: {0:?}")]
    Multiple(Box<Vec<Self>>),

    #[error("io error: {0}")]
    IO(#[from] std::io::Error),

    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),
    #[error("create bucket error")]
    S3Error(#[from] s3::error::S3Error),
    #[error("utf8 error")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("unknown store type '{0}'")]
    UnknownStore(String),
    #[error("invalid schema '{0}' expected '{1}'")]
    InvalidScheme(String, String),
    #[error("other: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Route {
    pub start: Option<u8>,
    pub end: Option<u8>,
    pub url: String,
}

impl Route {
    pub fn url<S: Into<String>>(s: S) -> Self {
        Self {
            start: None,
            end: None,
            url: s.into(),
        }
    }
}
/// The store trait defines a simple (low level) key/value store interface to set/get blobs
/// the concern of the store is to only store given data with given key and implement
/// the means to retrieve it again once a get is called.
#[async_trait::async_trait]
pub trait Store: Send + Sync + 'static {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>>;
    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()>;
    fn routes(&self) -> Vec<Route>;
}

/// The store factory works as a factory for a specific store
/// this is only needed to be able dynamically create different types
/// of stores based only on scheme of the store url.
/// the Factory returns a factory future that resolved to a Box<dyn Store>
pub type Factory = fn(u: &str) -> FactoryFuture;

/// FactoryFuture is a future that resolves to a Result<Box<dyn Store>> this
/// is returned by a factory function like above
pub type FactoryFuture = Pin<Box<dyn Future<Output = Result<Box<dyn Store>>>>>;

/// Router holds a set of shards (stores) where each store can be configured to serve
/// a range of hashes.
///
/// On get, all possible stores that is configured to serve this key are tried until the first
/// one succeed
///
/// On set, the router set the object on all matching stores, and fails if at least
/// one store fails, or if no store matches the key
pub type Router = router::Router<Box<dyn Store>>;

#[async_trait::async_trait]
impl Store for Router {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        if key.is_empty() {
            return Err(Error::InvalidKey);
        }
        let mut errors = Vec::default();

        // to make it fare we shuffle the list of matching routers randomly everytime
        // before we do a get
        let mut routers: Vec<&Box<dyn Store>> = self.route(key[0]).collect();
        routers.shuffle(&mut rand::thread_rng());
        for store in routers {
            match store.get(key).await {
                Ok(object) => return Ok(object),
                Err(err) => errors.push(err),
            };
        }

        if errors.is_empty() {
            return Err(Error::KeyNotRoutable);
        }

        // return aggregated errors
        return Err(Error::Multiple(Box::new(errors)));
    }

    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        if key.is_empty() {
            return Err(Error::InvalidKey);
        }

        let mut b = false;
        for store in self.route(key[0]) {
            b = true;
            store.set(key, blob).await?;
        }

        if !b {
            return Err(Error::KeyNotRoutable);
        }

        Ok(())
    }

    fn routes(&self) -> Vec<Route> {
        let mut routes = Vec::default();
        for (key, value) in self.routes.iter() {
            for sub in value.routes() {
                let r = Route {
                    start: Some(sub.start.unwrap_or(*key.start())),
                    end: Some(sub.end.unwrap_or(*key.end())),
                    url: sub.url,
                };
                routes.push(r);
            }
        }

        routes
    }
}

#[async_trait::async_trait]
impl Store for Box<dyn Store> {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        self.as_ref().get(key).await
    }
    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        self.as_ref().set(key, blob).await
    }
    fn routes(&self) -> Vec<Route> {
        self.as_ref().routes()
    }
}
