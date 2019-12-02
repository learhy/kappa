use std::net::SocketAddr;
use std::time::Duration;
use crate::capture::flow::Protocol;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Process {
    pub comm:      String,
    pub cmdline:   Vec<String>,
    pub cgroups:   Vec<CGroup>,
    pub pid:       u32,
    pub container: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct CGroup {
    pub hierarchy:   u32,
    pub controllers: Vec<String>,
    pub path:        String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub kind:  Kind,
    pub proto: Protocol,
    pub src:   SocketAddr,
    pub dst:   SocketAddr,
    pub srtt:  Duration,
    pub proc:  Process,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    Connect,
    Accept,
    TX,
    RX,
    Close,
}

pub use monitor::Procs;
pub use sockets::Sockets;

mod sockets;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod monitor;

#[cfg(not(target_os = "linux"))]
#[path = "monitor.rs"]
mod monitor;
