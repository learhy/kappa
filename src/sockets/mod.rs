use std::net::SocketAddr;
use std::time::Duration;
use libc::pid_t;
use serde::{Serialize, Deserialize};
use crate::capture::flow::Protocol;

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub kind:  Kind,
    pub pid:   pid_t,
    pub proto: Protocol,
    pub src:   SocketAddr,
    pub dst:   SocketAddr,
    pub srtt:  Duration,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    Connect,
    Accept,
    TX,
    RX,
    Close,
}

pub use monitor::Monitor;
pub use sockets::Sockets;

mod sockets;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod monitor;

#[cfg(not(target_os = "linux"))]
#[path = "monitor.rs"]
mod monitor;
