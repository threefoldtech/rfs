use super::{Error, Result, Route, Store};
use reqwest::{self, StatusCode};
use url::Url;

#[derive(Clone)]
pub struct HTTPStore {
    url: Url,
}

impl HTTPStore {
    pub async fn make<U: AsRef<str>>(url: &U) -> Result<HTTPStore> {
        let u = Url::parse(url.as_ref())?;
        if u.scheme() != "http" && u.scheme() != "https" {
            return Err(Error::Other(anyhow::Error::msg("invalid scheme")));
        }

        Ok(HTTPStore::new(u).await?)
    }
    pub async fn new<U: Into<Url>>(url: U) -> Result<Self> {
        let url = url.into();
        Ok(Self { url })
    }
}

#[async_trait::async_trait]
impl Store for HTTPStore {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        let file = hex::encode(key);
        let mut file_path = self.url.clone();
        file_path
            .path_segments_mut()
            .map_err(|_| Error::Other(anyhow::Error::msg("cannot be base")))?
            .push(&file[0..2])
            .push(&file);
        let mut legacy_path = self.url.clone();

        legacy_path
            .path_segments_mut()
            .map_err(|_| Error::Other(anyhow::Error::msg("cannot be base")))?
            .push(&file);

        let data = match reqwest::get(file_path).await {
            Ok(mut response) => {
                if response.status() == StatusCode::NOT_FOUND {
                    response = reqwest::get(legacy_path)
                        .await
                        .map_err(|_| Error::KeyNotFound)?;
                    if response.status() != StatusCode::OK {
                        return Err(Error::KeyNotFound);
                    }
                }
                if response.status() != StatusCode::OK {
                    return Err(Error::Unavailable);
                }
                response.bytes().await.map_err(|e| Error::Other(e.into()))?
            }
            Err(err) => return Err(Error::Other(err.into())),
        };
        Ok(data.into())
    }

    async fn set(&self, _key: &[u8], _blob: &[u8]) -> Result<()> {
        Err(Error::Other(anyhow::Error::msg(
            "http store doesn't support uploading",
        )))
    }

    fn routes(&self) -> Vec<Route> {
        let r = Route::url(self.url.clone());

        vec![r]
    }
}
