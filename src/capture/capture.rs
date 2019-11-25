use std::convert::TryInto;
use anyhow::{Result, anyhow};
use log::{info, warn};
use pcap::{Capture, Active};
use crate::capture::{Config, Sample, sample};

pub fn capture(name: &str, dev: &str, cfg: &Config) -> Result<Capture<Active>> {
    let mut cap = Capture::from_device(dev)?
        .buffer_size(cfg.buffer_size as i32)
        .timeout(cfg.interval.as_millis().try_into()?)
        .snaplen(cfg.snaplen as i32)
        .promisc(cfg.promisc)
        .open()?;

    match cap.list_datalinks()?.into_iter().find(|lt| lt.0 == 1) {
        Some(linktype) => cap.set_datalink(linktype)?,
        None           => return Err(anyhow!("not ethernet")),
    }

    if let Sample::Rate(n) = cfg.sample {
        match sample(&cap, n) {
            Ok(()) => info!("sampling {} at 1:{}", name, n),
            Err(e) => warn!("sampling {} failed: {}", name, e),
        }
    }

    Ok(cap)
}
