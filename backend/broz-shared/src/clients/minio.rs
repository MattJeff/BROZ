use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

#[derive(Clone)]
pub struct MinioClient {
    client: S3Client,
    bucket: String,
    public_url: String,
}

impl MinioClient {
    pub async fn new(
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        bucket: &str,
        public_url: &str,
    ) -> Self {
        let credentials = Credentials::new(access_key, secret_key, None, None, "minio");

        let config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(endpoint)
            .region(Region::new("us-east-1"))
            .credentials_provider(credentials)
            .force_path_style(true)
            .build();

        let client = S3Client::from_conf(config);

        // Ensure bucket exists
        let _ = client
            .create_bucket()
            .bucket(bucket)
            .send()
            .await;

        tracing::info!(endpoint = %endpoint, bucket = %bucket, "MinIO client initialized");

        Self {
            client,
            bucket: bucket.to_string(),
            public_url: public_url.to_string(),
        }
    }

    /// Upload a file and return the public URL
    pub async fn upload(
        &self,
        key: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<String, String> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body.into())
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| format!("upload failed: {e}"))?;

        Ok(format!("{}/{}/{}", self.public_url, self.bucket, key))
    }

    /// Generate a presigned URL for downloading
    pub async fn presigned_url(&self, key: &str, expires_secs: u64) -> Result<String, String> {
        let presign_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(expires_secs))
            .build()
            .map_err(|e| format!("presign config error: {e}"))?;

        let url = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presign_config)
            .await
            .map_err(|e| format!("presign error: {e}"))?
            .uri()
            .to_string();

        Ok(url)
    }

    /// Delete an object
    pub async fn delete(&self, key: &str) -> Result<(), String> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| format!("delete failed: {e}"))?;

        Ok(())
    }
}
