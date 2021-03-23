use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use anyhow::Result;
use nixv::Version;
use super::Sockets;

pub struct Procs {
    socks: Arc<Sockets>
}

impl Procs {
    pub fn new(_kernel: Option<Version>, _code: Option<Vec<u8>>) -> Result<Self> {
        Ok(Procs {
            socks: Arc::new(Sockets::new())
        })

    }

    pub fn watch(&mut self, _shutdown: Arc<AtomicBool>) -> Result<()> {
        Ok(())
    }

    pub fn sockets(&self) -> Arc<Sockets> {
        self.socks.clone()
    }
}
