use anyhow::{anyhow, Result};
use libc::pid_t;
use super::Process;

impl Process {
    pub async fn scan() -> Result<Vec<Self>> {
        Ok(Vec::new())
    }

    pub fn load(_pid: pid_t) -> Result<Self> {
        Err(anyhow!("unsupported function"))
    }
}
