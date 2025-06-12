use super::{Error, Result, Route, Store};

use anyhow::Context;
use s3::{creds::Credentials, error::S3Error, Bucket, Region};
use url::Url;

fn get_config<U: AsRef<str>>(u: U) -> Result<(Credentials, Region, String)> {
    let url = Url::parse(u.as_ref())?;

    let access_key = url.username().to_string();
    let access_secret = url.password().map(|s| s.to_owned());

    let host = url.host_str().context("host not found")?;
    let port = url.port().context("port not found")?;
    let scheme = match url.scheme() {
        "s3" => "http://",
        "s3+tls" | "s3s" => "https://",
        _ => return Err(Error::Other(anyhow::Error::msg("invalid scheme"))),
    };

    let endpoint = format!("{}{}:{}", scheme, host, port);

    let bucket_name = url.path().trim_start_matches('/').to_string();

    let region_name = url
        .query_pairs()
        .find(|(key, _)| key == "region")
        .map(|(_, value)| value.to_string())
        .unwrap_or_default();

    Ok((
        Credentials {
            access_key: Some(access_key),
            secret_key: access_secret,
            security_token: None,
            session_token: None,
            expiration: None,
        },
        Region::Custom {
            region: region_name,
            endpoint,
        },
        bucket_name,
    ))
}

#[derive(Clone)]
pub struct S3Store {
    bucket: Bucket,
    url: String,
    // this is only here as a work around for this bug https://github.com/durch/rust-s3/issues/337
    // because rfs uses the store in async (and parallel) matter to upload/download blobs
    // we need to synchronize this locally in that store which will hurt performance
    // the 2 solutions now is to either wait until this bug is fixed, or switch to another client
    // but for now we keep this work around
}

impl S3Store {
    pub async fn make<U: AsRef<str>>(url: &U) -> Result<S3Store> {
        let (cred, region, bucket_name) = get_config(url.as_ref())?;
        Ok(S3Store::new(url.as_ref(), &bucket_name, region, cred)?)
    }
    pub fn new(url: &str, bucket_name: &str, region: Region, cred: Credentials) -> Result<Self> {
        let bucket = Bucket::new(bucket_name, region, cred)
            .context("failed instantiate bucket")?
            .with_path_style();

        Ok(Self {
            bucket,
            url: url.to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl Store for S3Store {
    async fn get(&self, key: &[u8]) -> super::Result<Vec<u8>> {
        match self.bucket.get_object(hex::encode(key)).await {
            Ok(res) => Ok(res.to_vec()),
            Err(S3Error::HttpFailWithBody(404, _)) => Err(Error::KeyNotFound),
            Err(S3Error::Io(err)) => Err(Error::IO(err)),
            Err(err) => Err(anyhow::Error::from(err).into()),
        }
    }

    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        self.bucket
            .put_object(hex::encode(key), blob)
            .await
            .context("put object over s3 storage")?;

        Ok(())
    }

    fn routes(&self) -> Vec<Route> {
        vec![Route::url(self.url.clone())]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_config() {
        let (cred, region, bucket_name) =
            get_config("s3s://minioadmin:minioadmin@127.0.0.1:9000/mybucket?region=minio").unwrap();
        assert_eq!(
            cred,
            Credentials {
                access_key: Some("minioadmin".to_string()),
                secret_key: Some("minioadmin".to_string()),
                security_token: None,
                session_token: None,
                expiration: None,
            }
        );
        assert_eq!(
            region,
            Region::Custom {
                region: "minio".to_string(),
                endpoint: "https://127.0.0.1:9000".to_string()
            }
        );
        assert_eq!(bucket_name, "mybucket".to_string())
    }

    #[test]
    fn test_get_config_without_tls() {
        let (cred, region, bucket_name) =
            get_config("s3://minioadmin:minioadmin@127.0.0.1:9000/mybucket?region=minio").unwrap();
        assert_eq!(
            cred,
            Credentials {
                access_key: Some("minioadmin".to_string()),
                secret_key: Some("minioadmin".to_string()),
                security_token: None,
                session_token: None,
                expiration: None,
            }
        );
        assert_eq!(
            region,
            Region::Custom {
                region: "minio".to_string(),
                endpoint: "http://127.0.0.1:9000".to_string()
            }
        );
        assert_eq!(bucket_name, "mybucket".to_string())
    }

    #[ignore]
    #[tokio::test]
    async fn test_set_get() {
        let url = "s3://minioadmin:minioadmin@127.0.0.1:9000/mybucket?region=minio";
        let (cred, region, bucket_name) = get_config(url).unwrap();

        let store = S3Store::new(url, &bucket_name, region, cred);
        let store = store.unwrap();

        let key = b"test.txt";
        let blob = b"# Hello, World!";

        _ = store.set(key, blob).await;

        let get_res = store.get(key).await;
        let get_res = get_res.unwrap();

        assert_eq!(get_res, blob)
    }

    #[ignore]
    #[tokio::test]
    async fn test_set_get_without_region() {
        let url = "s3://minioadmin:minioadmin@127.0.0.1:9000/mybucket";
        let (cred, region, bucket_name) = get_config(url).unwrap();

        let store = S3Store::new(url, &bucket_name, region, cred);
        let store = store.unwrap();

        let key = b"test2.txt";
        let blob = b"# Hello, World!";

        _ = store.set(key, blob).await;

        let get_res = store.get(key).await;
        let get_res = get_res.unwrap();

        assert_eq!(get_res, blob)
    }
}
