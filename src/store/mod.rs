mod bs;
pub mod dir;
mod router;
pub mod zdb;

pub use bs::BlockStore;

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

    // TODO: better display for the Box<Vec<Self>>
    #[error("multiple error: {0:?}")]
    Multiple(Box<Vec<Self>>),

    #[error("io error: {0}")]
    IO(#[from] std::io::Error),

    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),
    #[error("other: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The store trait defines a simple (low level) key/value store interface to set/get blobs
/// the concern of the store is to only store given data with given key and implement
/// the means to retrieve it again once a get is called.
#[async_trait::async_trait]
pub trait Store: Send + Sync + 'static {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>>;
    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()>;
}

/// The store factory trait works as a factory for a specific store
/// this is only needed to be able dynamically create different types
/// of stores based only on scheme of the store url.
#[async_trait::async_trait]
pub trait StoreFactory {
    type Store: Store;

    async fn new<U: AsRef<str> + Send>(&self, url: U) -> anyhow::Result<Self::Store>;
}

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
        if key.len() == 0 {
            return Err(Error::InvalidKey);
        }
        let mut errors = Vec::default();
        for store in self.route(key[0]) {
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
        if key.len() == 0 {
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
}
