pub mod agent;
pub mod agg;
pub mod probe;

pub mod args;
pub mod capture;
pub mod collect;
pub mod combine;
pub mod export;
pub mod link;
pub mod os;
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
