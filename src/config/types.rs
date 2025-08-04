use anyhow::anyhow;

pub struct BufferSizeLimitKb(u64);

impl BufferSizeLimitKb {
    pub fn new(size_limit: u64) -> anyhow::Result<Self> {
        if size_limit == 0 {
            return Err(anyhow!("Value must be larger than 0"));
        } else if size_limit > 50_000 {
            return Err(anyhow!("Value must be smaller than 50,000 (50MB)"));
        }
        Ok(Self(size_limit))
    }

    pub fn inner(&self) -> u64 {
        self.0
    }
}
pub struct ObjectSizeLimitMb(u64);

impl ObjectSizeLimitMb {
    pub fn new(size_limit: u64) -> anyhow::Result<Self> {
        if size_limit == 0 {
            return Err(anyhow!("Value must be larger than 0"));
        } else if size_limit > 50_000 {
            return Err(anyhow!("Value must be smaller than 50,000 (50GB)"));
        }
        Ok(Self(size_limit))
    }

    pub fn inner(&self) -> u64 {
        self.0
    }
}
pub struct CronIntervalInMs(u64);

impl CronIntervalInMs {
    pub fn new(interval: u64) -> anyhow::Result<Self> {
        if interval == 0 {
            return Err(anyhow!("Value must be larger than 0"));
        }
        Ok(Self(interval))
    }

    pub fn inner(&self) -> u64 {
        self.0
    }
}
pub struct Bucket<'a>(pub Option<&'a str>);
pub struct Prefix<'a>(pub &'a str);
pub struct Postfix<'a>(pub &'a str);
pub struct Endpoint<'a>(pub Option<&'a str>);
