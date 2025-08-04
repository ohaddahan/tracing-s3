use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::ChecksumAlgorithm;

/// Helper utilities for S3 operations.
/// Provides methods for file size retrieval and appending content to S3 objects.
pub struct S3Helpers {}

impl S3Helpers {
    /// Retrieves the size of a file in S3.
    /// 
    /// # Arguments
    /// * `client` - The AWS S3 client
    /// * `bucket` - The S3 bucket name
    /// * `key` - The S3 object key
    /// 
    /// # Returns
    /// * `Ok(i64)` - The file size in bytes, or 0 if the file doesn't exist
    /// * `Err(anyhow::Error)` - If the head object operation fails
    pub async fn get_file_size(client: &Client, bucket: &str, key: &str) -> anyhow::Result<i64> {
        let resp = client.head_object().bucket(bucket).key(key).send().await?;
        Ok(resp.content_length.unwrap_or(0))
    }
    /// Appends content to an existing S3 object or creates a new one if it doesn't exist.
    /// Uses S3's write_offset_bytes feature for efficient appending.
    /// 
    /// # Arguments
    /// * `client` - The AWS S3 client
    /// * `bucket` - The S3 bucket name
    /// * `key` - The S3 object key
    /// * `content_to_append` - The content to append to the file
    /// 
    /// # Returns
    /// * `Ok(u64)` - The total file size after appending
    /// * `Err(anyhow::Error)` - If the append operation fails
    pub async fn append_to_file(
        client: &Client,
        bucket: &str,
        key: &str,
        content_to_append: &str,
    ) -> anyhow::Result<u64> {
        let offset = Self::get_file_size(client, bucket, key).await.unwrap_or(0);
        let total_len = offset as u64 + content_to_append.len() as u64;
        let content_to_append = content_to_append.as_bytes().to_vec();
        client
            .put_object()
            .set_write_offset_bytes(Some(offset))
            .checksum_algorithm(ChecksumAlgorithm::Crc64Nvme)
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(content_to_append)) // Empty byte array
            .send()
            .await?;
        Ok(total_len)
    }
}

#[cfg(test)]
mod tests {

    use crate::config::tracing_s3_config::TracingS3Config;
    use crate::config::types::{
        Bucket, BufferSizeLimitKb, CronIntervalInMs, Endpoint, ObjectSizeLimitMb, Postfix, Prefix,
    };
    use crate::s3_helpers::S3Helpers;
    use chrono::Utc;

    #[tokio::test]
    pub async fn append_to_file_test() {
        let config = TracingS3Config::new(
            None,
            None,
            None,
            Bucket(None),
            Prefix("prefix"),
            Postfix("log"),
            Endpoint(None),
            ObjectSizeLimitMb::new(1).unwrap(),
            CronIntervalInMs::new(1_000).unwrap(),
            BufferSizeLimitKb::new(1).unwrap(),
        )
        .await
        .unwrap();
        for _ in 0..5 {
            let _result = S3Helpers::append_to_file(
                &config.aws_client,
                &config.bucket,
                "check-file-exists.log",
                &format!("hello world {}\n", Utc::now()),
            )
            .await;
        }
    }
}
