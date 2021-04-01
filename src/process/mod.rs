use libc::pid_t;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Process {
    pub pid:       pid_t,
    pub comm:      String,
    pub cmdline:   Vec<String>,
    pub cgroups:   Vec<CGroup>,
    pub container: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct CGroup {
    pub hierarchy:   u32,
    pub controllers: Vec<String>,
    pub path:        String,
}

pub use procs::Procs;

mod procs;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod ext;

#[cfg(not(target_os = "linux"))]
#[path = "posix.rs"]
mod ext;
