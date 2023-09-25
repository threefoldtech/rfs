mod s3;

use rusoto_core::{Region, RusotoError};
use rusoto_s3::{S3, S3Client, CreateBucketRequest, PutObjectRequest, GetObjectRequest, CreateBucketError};
use tokio::io::AsyncReadExt;

struct BucketManager {
    client: S3Client,
    bucket: String,
}

struct Credentials {
    access_key: String,
    secret_key: String,
}

impl BucketManager{
    pub async fn new(endpoint: &str, region: &str, bucket: &str, cred: Credentials) -> Result<Self, String> {
        let region = Region::Custom {
            name: region.to_owned(),
            endpoint: endpoint.to_owned(),
        };

        let dispatcher = match rusoto_core::request::HttpClient::new() {
            Ok(http_client) => http_client,
            Err(err) => return Err(format!("Error creating http client: {:?}", err))
        };

        let client = S3Client::new_with(
            dispatcher,
            rusoto_core::credential::StaticProvider::new_minimal(cred.access_key.to_string(), cred.secret_key.to_string()),
            region,
        );

        // create the bucket if not there
        let create_bucket_request = CreateBucketRequest{
            bucket: bucket.to_owned(),
            ..Default::default()
        };

        match client.create_bucket(create_bucket_request).await {
            Ok(_) => Ok(Self {
                        client,
                        bucket: bucket.to_owned(),
                    }),
            Err(err) => {
                if let RusotoError::Service(CreateBucketError::BucketAlreadyOwnedByYou(_)) = err {
                    Ok(Self {
                        client,
                        bucket: bucket.to_owned(),
                    })
                } else {
                    Err(format!("Error creating bucket: {:?}", err))
                }
            }
        }
    }

    pub async fn set(&self, key: &str, data: &[u8]) -> Result<(), String> {
        let put_object_request = PutObjectRequest {
            bucket: self.bucket.to_owned(),
            key: key.to_owned(),
            body: Some(data.to_vec().into()),
            ..Default::default()
        };
        match self.client.put_object(put_object_request).await {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("Error uploading: {:?}", err)),
        }
    }

    pub async fn get(&self, key: &str) -> Result<Vec<u8>, String> {
        let get_object_request = GetObjectRequest {
            bucket: self.bucket.to_owned(),
            key: key.to_owned(),
            ..Default::default()
        };

        match self.client.get_object(get_object_request).await {
            Ok(response) => {
                if let Some(body) = response.body {
                    let mut buffer = Vec::new();
                    if let Err(io_err) = body.into_async_read().read_to_end(&mut buffer).await {
                        return Err(format!("Error reading data: {:?}", io_err))
                    } else {
                        return Ok(buffer);
                    }
                } else {
                    return Err("No data found in S3 object.".to_string())
                }
            }
            Err(err) => return Err(format!("Error retrieving data: {:?}", err)),
        }
    }
}