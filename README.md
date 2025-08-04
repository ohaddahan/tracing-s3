# S3 Tracing

A Rust crate that provides a [tracing](https://github.com/tokio-rs/tracing) layer for efficiently sending structured
logs to [AWS S3 Express One Zone buckets](https://aws.amazon.com/s3/storage-classes/express-one-zone).

## Docs

* [crates.io](https://crates.io/crates/tracing-s3)
* [docs.rs](https://docs.rs/tracing-s3/latest/tracing_s3/)

## Installation

`cargo add tracing-s3`

## Features

- **High Performance**: Optimized
  for [AWS S3 Express One Zone storage](https://aws.amazon.com/s3/storage-classes/express-one-zone/) class for ultra-low
  latency
- **Buffered Logging**: Smart buffering with configurable size limits and automatic flushing
- **Structured Output**: JSON-formatted logs with timestamps and span timing information
- **Configurable**: Environment variable support with programmatic overrides
- **Async/Tokio Compatible**: Built for modern async Rust applications
- **Automatic Partitioning**: Splits large log files into multiple parts when size limits are reached

## Quick Start

```rust
use tracing_s3::{HttpLogLayer, TracingS3Config};
use tracing_s3::config::types::*;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, Registry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure S3 logging
    let config = TracingS3Config::new(
        Some("us-west-2"),                          // AWS region
        Some("your-access-key"),                    // AWS access key
        Some("your-secret-key"),                    // AWS secret key
        Bucket(Some("your-express-bucket")),        // S3 Express bucket name
        Prefix("app-logs"),                         // Log file prefix
        Postfix("log"),                             // Log file extension
        Endpoint(None),                             // Custom endpoint (optional)
        ObjectSizeLimitMb::new(100)?,               // Max file size in MB
        CronIntervalInMs::new(5000)?,               // Flush interval in ms
        BufferSizeLimitKb::new(1024)?,              // Buffer size in KB
    ).await?;

    // Create the tracing layer
    let s3_layer = HttpLogLayer::new(Arc::new(config));
    
    // Set up tracing subscriber
    let subscriber = Registry::default()
        .with(s3_layer)
        .with(tracing_subscriber::fmt::layer());
    
    tracing::subscriber::set_global_default(subscriber)?;

    // Your application code with tracing
    tracing::info!("Application started");
    tracing::warn!("This is a warning");
    
    Ok(())
}
```

## Environment Variables

The crate supports the following environment variables:

- `S3_TRACING_AWS_REGION` - AWS region (default: "us-west-2")
- `S3_TRACING_BUCKET` - S3 bucket name
- `S3_TRACING_AWS_ACCESS_KEY_ID` - AWS access key ID
- `S3_TRACING_AWS_SECRET_ACCESS_KEY` - AWS secret access key

## Log Format

Logs are stored as JSON objects with the following structure:

```json
{
  "timestamp": "2024-01-01T12:00:00.000Z",
  "level": "INFO",
  "event": {
    "fields": {
      "message": "Your log message"
    },
    "target": "your_app",
    "span": {
      "name": "request_handler"
    }
  }
}
```

## File Organization

Log files are organized by date and partitioned when they exceed size limits:

```
2024-01-01/
├── 0/
│   └── app-logs-{uuid}.log
├── 1/
│   └── app-logs-{uuid}.log
└── ...
```

