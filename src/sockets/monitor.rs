use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use anyhow::Result;
use nixv::Version;
use super::Sockets;

pub struct Procs {
    socks: Arc<Sockets>
}

impl Procs {
    pub fn watch(_kernel: Option<Version>, _shutdown: Arc<AtomicBool>) -> Result<Self> {
        Ok(Procs {
            socks: Arc::new(Sockets::new())
        })
    }

    pub fn sockets(&self) -> Arc<Sockets> {
        self.socks.clone()
    }
}
