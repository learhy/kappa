use std::time::Duration;
use regex::Regex;

#[derive(Debug)]
pub struct Config {
    pub capture:     Regex,
    pub exclude:     Regex,
    pub interval:    Duration,
    pub buffer_size: u64,
    pub sample:      Sample,
    pub snaplen:     u64,
    pub promisc:     bool,
}

pub use capture::capture;
pub use decode::decode;
pub use flow::{Addr, Direction, Flow, Key, Protocol};
pub use sample::{sample, Sample};
pub use source::Sources;
pub use self::time::Timestamp;

pub mod flow;
pub mod queue;
pub mod time;
pub mod timer;

mod capture;
mod decode;
mod sample;
mod source;

#[cfg(test)]
mod test;
