pub mod probe;

pub mod args;
pub mod capture;
pub mod export;
pub mod link;
pub mod packet;
pub mod sockets;

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
