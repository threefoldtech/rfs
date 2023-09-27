use anyhow::{anyhow, Context, Result};
use rusoto_core::{Region, RusotoError};
use rusoto_s3::{
    CreateBucketError, CreateBucketRequest, GetObjectRequest, PutObjectRequest, S3Client, S3,
};
use tokio::io::AsyncReadExt;

#[derive(Clone)]
struct BucketManager {
    client: S3Client,
    bucket: String,
}

struct Credentials {
    access_key: String,
    secret_key: String,
}

impl BucketManager {
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
            rusoto_core::request::HttpClient::new().context("Error creating http client.")?;

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
                })
            }
            Err(err) => Err(err).context("Error creating bucket"),
        }
    }

    async fn set(&self, key: &str, data: &[u8]) -> Result<()> {
        let put_object_request = PutObjectRequest {
            bucket: self.bucket.clone(),
            key: key.to_owned(),
            body: Some(data.to_vec().into()),
            ..Default::default()
        };
        self.client
            .put_object(put_object_request)
            .await
            .context("Error uploading")?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        let get_object_request = GetObjectRequest {
            bucket: self.bucket.clone(),
            key: key.to_owned(),
            ..Default::default()
        };

        let res = self
            .client
            .get_object(get_object_request)
            .await
            .context("Error retrieving data")?;

        // ensure body in not none
        let body = res
            .body
            .ok_or_else(|| anyhow!("No data found in S3 object"))?;

        let mut buffer = Vec::new();
        body.into_async_read()
            .read_to_end(&mut buffer)
            .await
            .context("Error reading data")?;

        Ok(buffer)
    }
}
