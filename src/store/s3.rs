use super::{Error, Result, Route, Store};
use anyhow::Context;
use futures::Future;
use std::pin::Pin;

use rusoto_core::{ByteStream, Region, RusotoError};
use rusoto_s3::{
    CreateBucketError, CreateBucketRequest, GetObjectRequest, PutObjectRequest, S3Client, S3,
};
use tokio::io::AsyncReadExt;

fn get_config() -> Result<(String, String, Credentials)> {
    // TODO: get these from .env?
    Ok((
        String::from(""),
        String::from(""),
        Credentials {
            access_key: String::from(""),
            secret_key: String::from(""),
        },
    ))
}

async fn make_inner(url: String) -> Result<Box<dyn Store>> {
    let (region, bucket, cred) = get_config()?;
    // TODO: move creating the bucket here
    Ok(Box::new(S3Store::new(&url, &region, &bucket, cred).await?))
}

pub fn make(url: &str) -> Pin<Box<dyn Future<Output = Result<Box<dyn Store>>>>> {
    Box::pin(make_inner(url.into()))
}

#[derive(Clone)]
struct S3Store {
    client: S3Client,
    bucket: String,
    endpoint: String,
}

struct Credentials {
    access_key: String,
    secret_key: String,
}

impl S3Store {
    pub async fn new(
        endpoint: &str,
        region: &str,
        bucket: &str,
        cred: Credentials,
    ) -> Result<Self> {
        let region = Region::Custom {
            name: region.to_owned(),
            endpoint: endpoint.to_owned(),
        };

        let dispatcher =
            rusoto_core::request::HttpClient::new().context("failed to create http client.")?;

        let provider = rusoto_core::credential::StaticProvider::new_minimal(
            cred.access_key.clone(),
            cred.secret_key.clone(),
        );

        let client = S3Client::new_with(dispatcher, provider, region);

        let create_bucket_request = CreateBucketRequest {
            bucket: bucket.to_owned(),
            ..Default::default()
        };

        match client.create_bucket(create_bucket_request).await {
            Ok(_) | Err(RusotoError::Service(CreateBucketError::BucketAlreadyOwnedByYou(_))) => {
                Ok(Self {
                    client,
                    bucket: bucket.to_owned(),
                    endpoint: endpoint.to_owned(),
                })
            }
            Err(_) => return Err(Error::BucketCreationError),
        }
    }
}

#[async_trait::async_trait]
impl Store for S3Store {
    async fn get(&self, key: &[u8]) -> super::Result<Vec<u8>> {
        let get_object_request = GetObjectRequest {
            bucket: self.bucket.clone(),
            key: hex::encode(key),
            ..Default::default()
        };

        let res = self
            .client
            .get_object(get_object_request)
            .await
            .context("failed to get blob")?;

        let body = res.body.ok_or(Error::KeyNotFound)?;

        let mut buffer = Vec::new();
        if let Err(_) = body.into_async_read().read_to_end(&mut buffer).await {
            return Err(Error::InvalidBlob);
        }

        Ok(buffer)
    }

    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        let put_object_request = PutObjectRequest {
            bucket: self.bucket.clone(),
            key: hex::encode(key),
            body: Some(ByteStream::from(blob.to_owned())),
            ..Default::default()
        };
        self.client
            .put_object(put_object_request)
            .await
            .context("failed to set blob")?;

        Ok(())
    }

    fn routes(&self) -> Vec<Route> {
        vec![Route::url(self.endpoint.clone())]
    }
}
