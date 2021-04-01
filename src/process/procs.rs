use std::collections::HashMap;
use std::collections::hash_map::{VacantEntry, Entry};
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use libc::pid_t;
use log::{debug, error};
use parking_lot::Mutex;
use tokio::runtime::Handle;
use tokio::time::{interval, Instant};
use super::Process;

#[derive(Clone)]
pub struct Procs {
    map: Arc<Mutex<HashMap<pid_t, Item>>>,
}

#[derive(Debug)]
pub struct Item {
    data: Arc<Process>,
    seen: Instant,
}

impl Procs {
    pub fn new() -> Self {
         Self {
            map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn exec(&self, handle: &Handle) {
        let map = self.map.clone();
        handle.spawn(async move {
            match scan(map).await {
                Ok(()) => debug!("scan finished"),
                Err(e) => error!("scan failed: {:?}", e),
            }
        });

        let map = self.map.clone();
        handle.spawn(async move {
            match reap(map).await {
                Ok(()) => debug!("reap finished"),
                Err(e) => error!("reap failed: {:?}", e),
            }
        });
    }

    pub fn get(&self, pid: pid_t) -> Option<Arc<Process>> {
        match self.map.lock().entry(pid) {
            Entry::Occupied(e) => Some(e.get().data.clone()),
            Entry::Vacant(e)   => load(pid, e)
        }
    }

    pub fn list(&self) -> Vec<Arc<Process>> {
        self.map.lock().values().map(|item| {
            item.data.clone()
        }).collect()
    }
}

async fn scan(map: Arc<Mutex<HashMap<pid_t, Item>>>) -> Result<()> {
    let mut interval = interval(Duration::from_secs(60));

    loop {
        let now = interval.tick().await;

        for proc in Process::scan().await? {
            map.lock().insert(proc.pid, Item {
                data: Arc::new(proc),
                seen: now,
            });
        }

        let count = map.lock().len();
        let time  = now.elapsed();

        debug!("scanned {} processes in {:?}", count, time);
    }
}

async fn reap(map: Arc<Mutex<HashMap<pid_t, Item>>>) -> Result<()> {
    let mut interval = interval(Duration::from_secs(60));

    loop {
        let now   = interval.tick().await;
        let delay = Duration::from_secs(60);
        map.lock().retain(|_, item| {
            now.saturating_duration_since(item.seen) < delay
        });
    }
}

fn load(pid: pid_t, entry: VacantEntry<'_, pid_t, Item>) -> Option<Arc<Process>> {
    let proc = Process::load(pid).ok()?;
    Some(entry.insert(Item {
        data: Arc::new(proc),
        seen: Instant::now(),
    }).data.clone())
}
