use std::str::FromStr;
use anyhow::Result;
use pcap::{Capture, Active};

#[derive(Debug)]
pub enum Sample {
    Rate(u32),
    None,
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
pub fn sample(_cap: &Capture<Active>, _rate: u32) -> Result<()> {
    Err(anyhow::anyhow!("unsupported"))
}

impl FromStr for Sample {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(':');
        let base = split.next().map(u32::from_str);
        let rate = split.next().map(u32::from_str);

        let base = base.ok_or_else(|| format!("missing base: {}", s))?;
        let rate = rate.ok_or_else(|| format!("missing rate: {}", s))?;

        match (base, rate) {
            (Ok(1), Ok(n)) => Ok(Sample::Rate(n)),
            (Ok(n), _    ) => Err(format!("invalid base: {}", n)),
            _              => Err(format!("invalid rate: {}", s)),
        }
    }
}
