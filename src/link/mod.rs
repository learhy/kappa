pub use pnet::util::MacAddr;

#[derive(Debug)]
pub enum Event {
    Add(String, Option<MacAddr>),
    Del(String),
}

pub use monitor::Links;

#[cfg(target_os = "linux")]
#[path = "linux/monitor.rs"]
mod monitor;

#[cfg(not(target_os = "linux"))]
#[path = "monitor.rs"]
mod monitor;
