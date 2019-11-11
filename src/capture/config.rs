use std::convert::TryInto;
use std::time::Duration;
use anyhow::Result;
use log::{info, warn};
use pcap::{Capture, Active};
use regex::Regex;
use super::Sample;

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

            if let Sample::Rate(n) = cfg.sample {
                match sample(&cap, n) {
                    Ok(()) => info!("sampling {} at 1:{}", link, n),
                    Err(e) => warn!("sampling {} failed: {}", link, e),
                }
            }

            return Ok(Some(cap))
        }
    }

    warn!("link {} not ethernet", link);

    Ok(None)
}

#[cfg(target_os = "linux")]
pub fn sample(cap: &Capture<Active>, rate: u32) -> Result<()> {
    use std::os::unix::io::AsRawFd;
    use bpf::{Op, Prog};

    Ok(bpf::attach_filter(cap.as_raw_fd(), Prog::new(vec![
        Op::new(0x20, 0, 0, 0xfffff038),
        Op::new(0x94, 0, 0, rate),
        Op::new(0x15, 0, 1, 0x00000001),
        Op::new(0x06, 0, 0, 0xffffffff),
        Op::new(0x06, 0, 0, 0000000000),
    ]))?)
}

#[cfg(not(target_os = "linux"))]
fn sample(_cap: &Capture<Active>, _rate: u32) -> Result<()> {
    Err(anyhow::anyhow!("unsupported"))
}
