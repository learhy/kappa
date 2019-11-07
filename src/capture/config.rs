use std::convert::TryInto;
use std::time::Duration;
use anyhow::Result;
use log::warn;
use pcap::{Capture, Active};
use regex::Regex;

#[derive(Debug)]
pub struct Config {
    pub capture:     Regex,
    pub exclude:     Regex,
    pub interval:    Duration,
    pub buffer_size: u64,
    pub snaplen:     u64,
    pub promisc:     bool,
}

pub fn capture(link: &str, cfg: &Config) -> Result<Option<Capture<Active>>> {
    let mut cap = Capture::from_device(link)?
        .buffer_size(cfg.buffer_size as i32)
        .timeout(cfg.interval.as_millis().try_into()?)
        .snaplen(cfg.snaplen as i32)
        .promisc(cfg.promisc)
        .open()?;

    for linktype in cap.list_datalinks()? {
        if linktype.0 == 1 {
            cap.set_datalink(linktype)?;
            return Ok(Some(cap))
        }
    }

    warn!("link {} not ethernet", link);

    Ok(None)
}
