use super::{Error, Result, Route, Store};

use futures::Future;
use std::pin::Pin;

use s3::{creds::Credentials, Bucket, Region};

fn get_config(url: &str) -> Result<(Credentials, Region, String)> {
    let url = url::Url::parse(url.as_ref())?;

    let (access_key, access_secret, endpoint, bucket_name, region_name) = match url.host() {
        Some(_) => {
            let access_key = url.username().to_string();
            let access_secret = url
                .password()
                .ok_or(Error::InvalidConfigs(String::from(
                    "did not find secret key",
                )))?
                .to_string();
            let host = url
                .host_str()
                .ok_or(Error::InvalidConfigs(String::from("did not find host")))?
                .to_string();
            let port = url
                .port()
                .ok_or(Error::InvalidConfigs(String::from("did not find port")))?;

            // rust-s3 implementation force tls unless it found `://` in the endpoint
            // check rust-s3/aws-region/src/region.rs #fn scheme
            let scheme = match url.query_pairs().find(|(key, _)| key == "tls") {
                Some((_, value)) if value == "false" => "http://",
                _ => "",
            };

            let endpoint = format!("{}{}:{}", scheme, host, port);

            let bucket_name = url.path().trim_start_matches('/').to_string();

            let region = match url.query_pairs().find(|(key, _)| key == "region") {
                Some((_, value)) => value.to_string(),
                None => return Err(Error::InvalidConfigs(String::from("did not find region"))),
            };

            (access_key, access_secret, endpoint, bucket_name, region)
        }
        None => {
            return Err(Error::InvalidConfigs(String::from(
                "failed parsing the url",
            )))
        }
    };

    Ok((
        Credentials {
            access_key: Some(access_key),
            secret_key: Some(access_secret),
            security_token: None,
            session_token: None,
            expiration: None,
        },
        Region::Custom {
            region: region_name,
            endpoint: endpoint,
        },
        bucket_name,
    ))
}

async fn make_inner(url: String) -> Result<Box<dyn Store>> {
    let (cred, region, bucket_name) = get_config(&url)?;
    Ok(Box::new(
        S3Store::new(&url, &bucket_name, region, cred).await?,
    ))
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
    pub async fn new(
        url: &str,
        bucket_name: &str,
        region: Region,
        cred: Credentials,
    ) -> Result<Self> {
        let bucket = Bucket::new(bucket_name, region, cred)?.with_path_style();

        Ok(Self {
            bucket: bucket,
            url: url.to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl Store for S3Store {
    async fn get(&self, key: &[u8]) -> super::Result<Vec<u8>> {
        let response = self.bucket.get_object(std::str::from_utf8(key)?).await?;
        Ok(response.to_vec())
    }

    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        self.bucket
            .put_object(std::str::from_utf8(key)?, blob)
            .await?;
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
                endpoint: "127.0.0.1:9000".to_string()
            }
        );
        assert_eq!(bucket_name, "mybucket".to_string())
    }

    #[test]
    fn test_get_config_without_tls() {
        let (cred, region, bucket_name) =
            get_config("s3://minioadmin:minioadmin@127.0.0.1:9000/mybucket?region=minio&tls=false")
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
                endpoint: "http://127.0.0.1:9000".to_string()
            }
        );
        assert_eq!(bucket_name, "mybucket".to_string())
    }

    #[ignore]
    #[tokio::test]
    async fn test_set_get() {
        let url = "s3://minioadmin:minioadmin@127.0.0.1:9000/mybucket?region=minio&tls=false";
        let (cred, region, bucket_name) = get_config(url).unwrap();

        let store = S3Store::new(url, &bucket_name, region, cred).await;
        let store = store.unwrap();

        let key = b"test.txt";
        let blob = b"# Hello, World!";

        _ = store.set(key, blob).await;

        let get_res = store.get(key).await;
        let get_res = get_res.unwrap();

        assert_eq!(get_res, blob)
    }
}
