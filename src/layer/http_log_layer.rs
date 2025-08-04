use crate::config::tracing_s3_config::TracingS3Config;
use crate::s3_helpers::S3Helpers;
use chrono::Local;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;
use uuid::Uuid;

/// Represents the output buffer for log data before it's sent to S3.
/// Manages buffering, naming, and partitioning of log files.
pub struct Output {
    name: String,
    size_in_bytes: Arc<AtomicU64>,
    buffer: Arc<RwLock<Vec<String>>>,
    part: Arc<AtomicU64>,
    prefix: String,
    postfix: String,
    nonce: String,
}

impl Output {
    /// Creates a new Output instance with the specified prefix and postfix.
    ///
    /// # Arguments
    /// * `prefix` - The prefix for log file names
    /// * `postfix` - The postfix/extension for log file names
    ///
    /// # Returns
    /// A new Output instance with initialized buffer and metadata
    pub fn new(prefix: &str, postfix: &str) -> Self {
        let nonce = Uuid::new_v4().to_string();
        Self {
            name: Self::gen_name(prefix, 0, postfix, &nonce),
            nonce,
            prefix: prefix.to_string(),
            postfix: postfix.to_string(),
            buffer: Arc::new(RwLock::new(Vec::new())),
            size_in_bytes: Arc::new(AtomicU64::new(0)),
            part: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increments the part number and updates the file name.
    /// Called when the current log file becomes too large and needs to be split.
    pub fn bump_part(&mut self) {
        self.part.fetch_add(1, Ordering::Relaxed);
        let name = Self::gen_name(&self.prefix, self.part(), &self.postfix, &self.nonce);
        self.update_name(&name);
    }

    /// Generates a log file name based on the current date, part number, and configuration.
    ///
    /// # Arguments
    /// * `prefix` - The file name prefix
    /// * `part` - The part number for file splitting
    /// * `postfix` - The file extension/postfix
    /// * `nonce` - A unique identifier for this logging session
    ///
    /// # Returns
    /// A formatted file name in the pattern: YYYY-MM-DD/part/prefix-nonce.postfix
    pub fn gen_name(prefix: &str, part: u64, postfix: &str, nonce: &str) -> String {
        let today = Local::now().date_naive();
        format!(
            "{}/{part}/{prefix}-{nonce}.{postfix}",
            today.format("%Y-%m-%d"),
        )
    }

    /// Returns the number of log entries currently in the buffer.
    pub async fn buffer_len(&self) -> u64 {
        self.buffer.read().await.len() as u64
    }

    /// Flushes the buffer and returns all buffered log entries as a single string.
    /// Clears the buffer and resets the size counter after flushing.
    ///
    /// # Returns
    /// A newline-delimited string containing all buffered log entries
    pub async fn flush_buffer(&self) -> String {
        let payload = self.buffer.read().await.join("\n");
        self.buffer.write().await.clear();
        self.update_size_in_bytes(0);
        payload
    }

    /// Appends a log entry to the buffer and updates the size counter.
    ///
    /// # Arguments
    /// * `value` - The log entry to append to the buffer
    pub async fn append_to_buffer(&self, value: String) {
        let size_in_kb = self
            .size_in_bytes
            .fetch_add(value.len() as u64, Ordering::Relaxed);
        let mut mut_buffer = self.buffer.write().await;
        mut_buffer.push(value);
        self.update_size_in_bytes(size_in_kb);
    }

    /// Returns the current log file name.
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns the current buffer size in bytes.
    pub fn size_in_bytes(&self) -> u64 {
        self.size_in_bytes.load(Ordering::Relaxed)
    }

    /// Returns the current part number.
    pub fn part(&self) -> u64 {
        self.part.load(Ordering::Relaxed)
    }

    /// Updates the buffer size counter.
    ///
    /// # Arguments
    /// * `size_in_bytes` - The new buffer size in bytes
    pub fn update_size_in_bytes(&self, size_in_bytes: u64) {
        self.size_in_bytes.store(size_in_bytes, Ordering::Relaxed);
    }

    /// Updates the current log file name.
    ///
    /// # Arguments
    /// * `name` - The new file name
    pub fn update_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
}

/// The main tracing layer that handles log collection and S3 uploading.
/// Implements the tracing-subscriber Layer trait to integrate with the tracing ecosystem.
pub struct HttpLogLayer {
    pub output: Arc<RwLock<Output>>,
    pub config: Arc<TracingS3Config>,
    pub handle_tx: UnboundedSender<JoinHandle<()>>,
    pub event_tx: UnboundedSender<Value>,
}

impl HttpLogLayer {
    /// Creates a background task that periodically flushes buffered logs to S3.
    ///
    /// # Arguments
    /// * `config` - The S3 configuration
    /// * `output` - The shared output buffer
    ///
    /// # Returns
    /// A JoinHandle for the background cron job task
    pub fn cron_job(config: Arc<TracingS3Config>, output: Arc<RwLock<Output>>) -> JoinHandle<()> {
        let buffer_size_limit_kb = config.buffer_size_limit_kb;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(config.cron_interval_in_ms)).await;
                let buffer_len = output.read().await.buffer_len().await;
                let size_in_bytes = output.read().await.size_in_bytes();
                if buffer_len > 0 || size_in_bytes * 1_024 >= buffer_size_limit_kb {
                    let _ = HttpLogLayer::send_logs(config.clone(), output.clone()).await;
                }
            }
        })
    }

    /// Creates a new HttpLogLayer instance.
    ///
    /// Sets up the background cron job for periodic log flushing and initializes
    /// the event processing pipeline.
    ///
    /// # Arguments
    /// * `config` - The S3 configuration wrapped in an Arc
    ///
    /// # Returns
    /// A new HttpLogLayer instance ready to receive tracing events
    pub fn new(config: Arc<TracingS3Config>) -> Self {
        let output = Arc::new(RwLock::new(Output::new(&config.prefix, &config.postfix)));
        let (handle_tx, mut handle_rx): (
            UnboundedSender<JoinHandle<()>>,
            UnboundedReceiver<JoinHandle<()>>,
        ) = mpsc::unbounded_channel();

        let (event_tx, mut event_rx): (UnboundedSender<Value>, UnboundedReceiver<Value>) =
            mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(handle) = handle_rx.recv().await {
                let _ = handle.await;
            }
        });
        let output_clone = output.clone();
        tokio::spawn(async move {
            while let Some(value) = event_rx.recv().await {
                if let Ok(v) = serde_json::to_string(&value) {
                    output_clone.read().await.append_to_buffer(v).await;
                }
            }
        });

        let cron_job_handle = Self::cron_job(config.clone(), output.clone());
        let _ = handle_tx.send(cron_job_handle);
        Self {
            output,
            handle_tx,
            config,
            event_tx,
        }
    }

    /// Sends buffered logs to S3 and handles file partitioning if necessary.
    ///
    /// # Arguments
    /// * `config` - The S3 configuration
    /// * `output` - The shared output buffer
    ///
    /// # Returns
    /// * `Ok(())` - If logs were successfully sent
    /// * `Err(anyhow::Error)` - If the S3 upload operation fails
    pub async fn send_logs(
        config: Arc<TracingS3Config>,
        output: Arc<RwLock<Output>>,
    ) -> anyhow::Result<()> {
        let payload = output.read().await.flush_buffer().await;
        let name = output.read().await.name();
        let total_size =
            S3Helpers::append_to_file(&config.aws_client, &config.bucket, &name, &payload).await?;
        if total_size > config.buffer_size_limit_kb * 1_024 {
            output.write().await.bump_part();
        }
        Ok(())
    }
}
