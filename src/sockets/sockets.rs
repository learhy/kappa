use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use log::debug;
use parking_lot::RwLock;
use crate::capture::flow::{Flow, Key, Protocol};
use crate::collect::Record;
use super::{Event, Kind, Process};
use Protocol::TCP;

pub struct Sockets {
    socks:   RwLock<HashMap<Key, Socket>>,
    timeout: Duration,
}

#[derive(Debug)]
pub struct Socket {
    proc:   Arc<Process>,
    closed: Option<Instant>,
}

impl Sockets {
    pub fn new() -> Self {
        Self {
            socks:   RwLock::new(HashMap::new()),
            timeout: Duration::from_secs(60),
        }
    }

    pub fn get(&self, key: &Key) -> Option<Arc<Process>> {
        self.socks.read().get(key).map(|e| e.proc.clone())
    }

    pub fn merge(&self, flow: Vec<Flow>) -> Vec<Record> {
        let socks = self.socks.read();
        flow.into_iter().map(|flow| {
            let src = socks.get(&Key(TCP, flow.src, flow.dst));
            let dst = socks.get(&Key(TCP, flow.dst, flow.src));
            Record {
                flow: flow,
                src:  src.map(|s| s.proc.clone()),
                dst:  dst.map(|s| s.proc.clone()),
            }
        }).collect()
    }

    pub fn update(&self, e: Event) {
        match e.kind {
            Kind::Accept  => self.insert(e),
            Kind::Connect => self.insert(e),
            Kind::Close   => self.finish(e),
        }
    }

    fn insert(&self, e: Event) {
        let key  = Key(TCP, e.src.into(), e.dst.into());
        let src  = e.src;
        let dst  = e.dst;
        let proc = e.proc;
        self.socks.write().entry(key).or_insert_with(|| {
            debug!("{} -> {}: {} ({})", src, dst, proc.comm, proc.pid);
            Socket {
                proc:   Arc::new(proc),
                closed: None,
            }
        });
    }

    fn finish(&self, e: Event) {
        let key = Key(TCP, e.src.into(), e.dst.into());
        if let Some(sock) = self.socks.write().get_mut(&key) {
            sock.closed = Some(Instant::now());
        }
    }

    pub fn compact(&self) {
        let now     = Instant::now();
        let expired = |i: Instant| i.saturating_duration_since(now) >= self.timeout;
        self.socks.write().retain(|_, s| {
            !s.closed.map(expired).unwrap_or(false)
        });
    }
}
