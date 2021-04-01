pub mod agent;
pub mod agg;
pub mod probe;

pub mod args;
pub mod augment;
pub mod capture;
pub mod collect;
pub mod combine;
pub mod export;
pub mod link;
pub mod os;
pub mod packet;
pub mod process;
pub mod sockets;

use anyhow::Result;
use nix::unistd::gethostname;

pub fn hostname() -> Result<String> {
    let mut buf = [0u8; 256];
    let cstr = gethostname(&mut buf)?;
    Ok(cstr.to_string_lossy().to_string())
}

pub mod chf_capnp {
    include!(concat!(env!("OUT_DIR"), "/chf_capnp.rs"));
}

#[cfg(target_os = "linux")]
pub mod probes;

#[cfg(not(target_os = "linux"))]
pub mod probes {
    pub fn clear() -> Result<(), String> {
        Ok(())
    }
}
