use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use anyhow::Result;
use nixv::Version;
use serde::{Serialize, Deserialize};
use super::Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Process {
    pub comm:      String,
    pub cmdline:   Vec<String>,
    pub cgroups:   Vec<CGroup>,
    pub pid:       u32,
    pub container: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CGroup {
    pub hierarchy:   u32,
    pub controllers: Vec<String>,
    pub path:        String,
}

pub struct Monitor;

impl Monitor {
    pub fn start(_kernel: Option<Version>, _shutdown: Arc<AtomicBool>) -> Result<Self> {
        Ok(Monitor)
    }

    pub fn recv(&mut self) -> Result<Option<Event>> {
        Ok(None)
    }
}
