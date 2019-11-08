use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use anyhow::Result;
use nixv::Version;
use super::Event;

pub struct Socks;

impl Socks {
    pub fn watch(_kernel: Option<Version>, _shutdown: Arc<AtomicBool>) -> Result<Self> {
        Ok(Socks)
    }

    pub fn recv(&mut self) -> Result<Option<Event>> {
        Ok(None)
    }
}
