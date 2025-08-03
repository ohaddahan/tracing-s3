use anyhow::anyhow;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{Client, Error};

pub async fn check_file_exists(client: &Client, bucket: &str, key: &str) -> anyhow::Result<bool> {
    let resp = client.head_object().bucket(bucket).key(key).send().await;

    match resp {
        Ok(_) => Ok(true),
        Err(e) => {
            if e.as_service_error()
                .map(|se| se.code() == Some("NotFound"))
                .unwrap_or(false)
            {
                Ok(false)
            } else {
                Err(anyhow!("{e}"))
            }
        }
    }
}

pub async fn touch_file(client: &Client, bucket: &str, key: &str) -> Result<(), Error> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(vec![])) // Empty byte array
        .send()
        .await?;

    println!("File {} created (touched) in bucket {}", key, bucket);
    Ok(())
}

pub async fn get_file_size(client: &Client, bucket: &str, key: &str) -> Result<i64, Error> {
    let resp = client.head_object().bucket(bucket).key(key).send().await?;
    Ok(resp.content_length.unwrap_or(0))
}
