use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::ChecksumAlgorithm;

pub struct S3Helpers {}

impl S3Helpers {
    pub async fn get_file_size(client: &Client, bucket: &str, key: &str) -> anyhow::Result<i64> {
        let resp = client.head_object().bucket(bucket).key(key).send().await?;
        Ok(resp.content_length.unwrap_or(0))
    }
    pub async fn append_to_file_multipart(
        client: &Client,
        bucket: &str,
        key: &str,
        content_to_append: &str,
    ) -> anyhow::Result<()> {
        let offset = Self::get_file_size(client, bucket, key).await.unwrap_or(0);
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
        Ok(())
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
            let result = S3Helpers::append_to_file_multipart(
                &config.aws_client,
                &config.bucket,
                "check-file-exists.log",
                &format!("hello world {}\n", Utc::now()),
            )
            .await;
            println!("append_to_file_test result => {result:?}");
        }
    }
}
