use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use anyhow::Result;
use nixv::Version;
use super::Sockets;

pub struct Monitor;

impl Monitor {
    pub fn new(_kernel: Option<Version>, _code: Option<Vec<u8>>) -> Result<Self> {
        Ok(Self)
    }

    pub fn watch(&mut self, _sockets: Arc<Sockets>, _shutdown: Arc<AtomicBool>) -> Result<()> {
        Ok(())
    }
}
