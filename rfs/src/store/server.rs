use super::{Error, Result, Route, Store};
use crate::server_api;
use reqwest::Client;
use std::sync::Arc;
use url;

pub const SCHEME: &str = "server";

/// ServerStore is a store that interfaces with the fl-server's API
/// It supports both uploads and downloads for blocks using the server's HTTP API
#[derive(Clone)]
pub struct ServerStore {
    /// Server URL
    server_url: String,
    /// HTTP client for making requests
    client: Arc<Client>,
    /// Authentication token
    token: Option<String>,
}

impl ServerStore {
    pub async fn make<U: AsRef<str>>(url: &U) -> Result<ServerStore> {
        let u = url::Url::parse(url.as_ref())?;
        if u.scheme() != SCHEME {
            return Err(Error::InvalidScheme(u.scheme().into(), SCHEME.into()));
        }

        // Extract the token from the query parameters
        let token = u
            .query_pairs()
            .find(|(key, _)| key == "token")
            .map(|(_, value)| value.to_string());

        // Extract the actual server URL (e.g., "http://localhost:4000")
        let server_url = u
            .host_str()
            .map(|host| format!("{}://{}", host, u.path().trim_start_matches('/')))
            .ok_or_else(|| Error::InvalidScheme("Invalid host in URL".into(), SCHEME.into()))?;

        let client = Arc::new(Client::new());

        Ok(Self {
            server_url,
            client,
            token,
        })
    }

    /// Create a new ServerStore with the given server URL
    pub fn new(server_url: String, token: Option<String>) -> Self {
        let client = Arc::new(Client::new());

        Self {
            server_url,
            client,
            token,
        }
    }
}

#[async_trait::async_trait]
impl Store for ServerStore {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        // Convert the key to a hex string
        let hash = hex::encode(key);

        // Download the block from the server
        match server_api::download_block(&hash, &self.server_url).await {
            Ok(data) => Ok(data.to_vec()),
            Err(err) => {
                // Check if the error is because the block doesn't exist
                if err.to_string().contains("404") {
                    return Err(Error::KeyNotFound);
                }
                Err(Error::Other(err))
            }
        }
    }

    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        // Convert the key to a hex string
        let hash = hex::encode(key);

        // Upload the block to the server
        let file_hash = "".to_string(); // Use the hash as the file hash for simplicity
        let idx = 0; // Use 0 as the index for testing

        server_api::upload_block(
            Arc::clone(&self.client),
            self.server_url.clone(),
            hash,
            blob.to_vec(),
            file_hash,
            idx,
            self.token.clone().unwrap_or_default(),
        )
        .await
        .map_err(|err| Error::Other(err))?;

        Ok(())
    }

    fn routes(&self) -> Vec<Route> {
        vec![Route::url(format!("{}://{}", SCHEME, self.server_url))]
    }
}
