mod router;
pub mod zdb;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("key not found")]
    KeyNotFound,
    #[error("invalid key")]
    InvalidKey,
    #[error("key is not routable")]
    KeyNotRoutable,
    #[error("store is not available")]
    Unavailable,

    // TODO: better display for the Box<Vec<Self>>
    #[error("multiple error: {0:?}")]
    Multiple(Box<Vec<Self>>),
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[error("other: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[async_trait::async_trait]
pub trait Store: Send + Sync + 'static {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>>;
    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()>;
}

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

        for store in self.route(key[0]) {
            store.set(key, blob).await?;
        }

        Ok(())
    }
}
