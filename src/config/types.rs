use anyhow::anyhow;

/// Represents a buffer size limit in kilobytes.
/// Used to control the maximum amount of log data to buffer in memory before flushing to S3.
pub struct BufferSizeLimitKb(u64);

impl BufferSizeLimitKb {
    /// Creates a new BufferSizeLimitKb instance.
    ///
    /// # Arguments
    /// * `size_limit` - The buffer size limit in kilobytes (1-50,000 KB)
    ///
    /// # Returns
    /// * `Ok(BufferSizeLimitKb)` - If the size limit is valid
    /// * `Err(anyhow::Error)` - If the size limit is 0 or greater than 50,000 KB
    pub fn new(size_limit: u64) -> anyhow::Result<Self> {
        if size_limit == 0 {
            return Err(anyhow!("Value must be larger than 0"));
        } else if size_limit > 50_000 {
            return Err(anyhow!("Value must be smaller than 50,000 (50MB)"));
        }
        Ok(Self(size_limit))
    }

    /// Returns the inner buffer size limit value in kilobytes.
    pub fn inner(&self) -> u64 {
        self.0
    }
}

/// Represents an object size limit in megabytes.
/// Used to control the maximum size of individual log files in S3 before creating a new part.
pub struct ObjectSizeLimitMb(u64);

impl ObjectSizeLimitMb {
    /// Creates a new ObjectSizeLimitMb instance.
    ///
    /// # Arguments
    /// * `size_limit` - The object size limit in megabytes (1-50,000 MB)
    ///
    /// # Returns
    /// * `Ok(ObjectSizeLimitMb)` - If the size limit is valid
    /// * `Err(anyhow::Error)` - If the size limit is 0 or greater than 50,000 MB
    pub fn new(size_limit: u64) -> anyhow::Result<Self> {
        if size_limit == 0 {
            return Err(anyhow!("Value must be larger than 0"));
        } else if size_limit > 50_000 {
            return Err(anyhow!("Value must be smaller than 50,000 (50GB)"));
        }
        Ok(Self(size_limit))
    }

    /// Returns the inner object size limit value in megabytes.
    pub fn inner(&self) -> u64 {
        self.0
    }
}

/// Represents a cron interval in milliseconds.
/// Used to control how frequently the background task flushes buffered logs to S3.
pub struct CronIntervalInMs(u64);

impl CronIntervalInMs {
    /// Creates a new CronIntervalInMs instance.
    ///
    /// # Arguments
    /// * `interval` - The cron interval in milliseconds (must be greater than 0)
    ///
    /// # Returns
    /// * `Ok(CronIntervalInMs)` - If the interval is valid
    /// * `Err(anyhow::Error)` - If the interval is 0
    pub fn new(interval: u64) -> anyhow::Result<Self> {
        if interval == 0 {
            return Err(anyhow!("Value must be larger than 0"));
        }
        Ok(Self(interval))
    }

    /// Returns the inner cron interval value in milliseconds.
    pub fn inner(&self) -> u64 {
        self.0
    }
}

/// Represents an S3 bucket name.
/// Can be provided directly or resolved from environment variables.
pub struct Bucket<'a>(pub Option<&'a str>);

/// Represents a prefix for log file names.
/// Used to organize and identify log files in the S3 bucket.
pub struct Prefix<'a>(pub &'a str);

/// Represents a postfix/extension for log file names.
/// Typically used for file extensions like "log" / "json" / "jsonl".
pub struct Postfix<'a>(pub &'a str);

/// Represents a custom S3 endpoint URL.
/// Optional parameter for using custom S3-compatible endpoints.
pub struct Endpoint<'a>(pub Option<&'a str>);
