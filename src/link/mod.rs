use std::fs::File;
use anyhow::Error;
use pnet::util::MacAddr;

#[derive(Debug)]
pub enum Event {
    Add(Add),
    Delete(String),
    Error(String, Error),
}

#[derive(Debug)]
pub struct Add {
    pub name:  String,
    pub dev:   String,
    pub mac:   Option<MacAddr>,
    pub netns: Option<File>,
}

pub use monitor::Links;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod monitor;

#[cfg(not(target_os = "linux"))]
#[path = "monitor.rs"]
mod monitor;
