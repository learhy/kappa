use std::net::SocketAddr;
use serde::{Serialize, Deserialize};

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


#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub proc: Process,
    pub kind: Kind,
    pub src:  SocketAddr,
    pub dst:  SocketAddr,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    Connect,
    Accept,
    Close,
}

pub use backend::*;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod backend;

#[cfg(not(target_os = "linux"))]
#[path = "empty.rs"]
mod backend;
