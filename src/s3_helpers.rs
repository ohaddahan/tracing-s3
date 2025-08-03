use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::ChecksumAlgorithm;

pub async fn get_file_size(client: &Client, bucket: &str, key: &str) -> anyhow::Result<i64> {
    let resp = client.head_object().bucket(bucket).key(key).send().await?;
    Ok(resp.content_length.unwrap_or(0))
}

async fn append_to_file_multipart(
    client: &Client,
    bucket: &str,
    key: &str,
    // Ensure newline between each line
    content_to_append: &str,
) -> anyhow::Result<()> {
    let offset = get_file_size(client, bucket, key)
        .await
        .unwrap_or_else(|_| 0);
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

#[cfg(test)]
mod tests {
    use crate::config::{Bucket, Config, CronIntervalInMs, Endpoint, ObjectSizeLimitMb, Prefix};
    use crate::s3_helpers::{append_to_file_multipart, get_file_size};
    use chrono::Utc;

    #[tokio::test]
    pub async fn append_to_file_test() {
        let config = Config::new(
            None,
            None,
            None,
            Bucket(None),
            Prefix("prefix"),
            Endpoint(None),
            ObjectSizeLimitMb(1),
            CronIntervalInMs(1_000),
        )
        .await
        .unwrap();
        for _ in 0..5 {
            let result = append_to_file_multipart(
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
