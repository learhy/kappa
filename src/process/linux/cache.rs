use std::collections::HashMap;
use std::collections::hash_map::{Entry, VacantEntry};
use std::time::{Duration, Instant};
use log::{log_enabled, trace, warn};
use log::Level::Trace;
use crate::process::Process;
use super::lookup;

pub struct Cache {
    procs:  HashMap<u32, Process>,
    loaded: Instant,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            procs:  HashMap::new(),
            loaded: Instant::now(),
        }
    }

    pub fn get(&mut self, pid: u32) -> Option<&Process> {
        if self.loaded.elapsed() > Duration::from_secs(30) {
            self.procs.clear();
            self.loaded = Instant::now();
        }

        match self.procs.entry(pid) {
            Entry::Occupied(e) => Some(e.into_mut()),
            Entry::Vacant(e)   => load(pid, e)
        }
    }
}

fn load<'a>(pid: u32, entry: VacantEntry<'a, u32, Process>) -> Option<&'a Process> {
    let proc = lookup(pid).map_err(|e| {
        warn!("failed to lookup {}: {:?}", pid, e);
    }).ok()?;

    if log_enabled!(Trace) {
        let exe = match proc.cmdline.first() {
            Some(name) => name.as_str(),
            None       => "",
        };
        trace!("pid {} is '{}': {}", pid, proc.comm, exe);
    }

    Some(entry.insert(proc))
}
