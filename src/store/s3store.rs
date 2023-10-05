use super::{Error, Result, Route, Store};

use anyhow::Context;
use futures::Future;
use std::pin::Pin;

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
        .context("region name not found")?;

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

async fn make_inner(url: String) -> Result<Box<dyn Store>> {
    let (cred, region, bucket_name) = get_config(&url)?;
    Ok(Box::new(S3Store::new(&url, &bucket_name, region, cred)?))
}

pub fn make(url: &str) -> Pin<Box<dyn Future<Output = Result<Box<dyn Store>>>>> {
    Box::pin(make_inner(url.into()))
}

#[derive(Clone)]
struct S3Store {
    bucket: Bucket,
    url: String,
}

impl S3Store {
    pub fn new(url: &str, bucket_name: &str, region: Region, cred: Credentials) -> Result<Self> {
        let bucket = Bucket::new(bucket_name, region, cred)
            .context("failed instantiate bucket")?
            .with_path_style();

        Ok(Self {
            bucket: bucket,
            url: url.to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl Store for S3Store {
    async fn get(&self, key: &[u8]) -> super::Result<Vec<u8>> {
        match self.bucket.get_object(hex::encode(key)).await {
            Ok(res) => Ok(res.to_vec()),
            Err(S3Error::Http(404, _)) => Err(Error::KeyNotFound),
            Err(S3Error::Io(err)) => Err(Error::IO(err)),
            Err(err) => Err(Error::Other(anyhow::Error::from(err))),
        }
    }

    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        match self.bucket.put_object(hex::encode(key), blob).await {
            Ok(_) => Ok(()),
            Err(err) => Err(Error::Other(anyhow::Error::from(err))),
        }
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
            get_config("s3s://minioadmin:minioadmin@127.0.0.1:9000/mybucket?region=minio")
                .unwrap();
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
}
