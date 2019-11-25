pub fn getpid() -> u32 {
    unsafe {
        libc::getpid() as u32
    }
}

pub use os::{findns, getns, setns};

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
pub mod os;

#[cfg(not(target_os = "linux"))]
pub mod os {
    use std::fs::File;
    use anyhow::Result;

    pub fn findns(_nsid: u32) -> Result<File> {
        unimplemented!();
    }

    pub fn getns(_pid: u32) -> Result<File> {
        unimplemented!();
    }

    pub fn setns(_ns: &File) -> Result<()> {
        unimplemented!();
    }
}
