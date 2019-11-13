use std::collections::HashMap;
use std::sync::Arc;
use log::debug;
use parking_lot::RwLock;
use crate::capture::flow::{Flow, Direction, Key, Protocol};
use super::{Event, Kind, Socket, Process};

pub struct Sockets {
    socks: RwLock<HashMap<Key, Socket>>,
}

impl Sockets {
    pub fn new() -> Self {
        Self {
            socks: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &Key) -> Option<Arc<Process>> {
        self.socks.read().get(key).map(|e| e.proc.clone())
    }

    pub fn merge(&self, flow: Vec<Flow>) -> Vec<(Flow, Option<Arc<Process>>)> {
        let socks = self.socks.read();
        flow.into_iter().map(|flow| {
            let src = flow.src;
            let dst = flow.dst;

            let key = match flow.direction {
                Direction::In                       => Key(Protocol::TCP, dst, src),
                Direction::Out | Direction::Unknown => Key(Protocol::TCP, src, dst),
            };

            let sock = socks.get(&key);

            (flow, sock.map(|e| e.proc.clone()))
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
        let key  = Key(Protocol::TCP, e.src.into(), e.dst.into());
        let src  = e.src;
        let dst  = e.dst;
        let proc = e.proc;
        self.socks.write().entry(key).or_insert_with(|| {
            debug!("{} -> {}: {} ({})", src, dst, proc.comm, proc.pid);
            Socket {
                proc:   Arc::new(proc),
                closed: false,
            }
        });
    }

    fn finish(&self, e: Event) {
        let key = Key(Protocol::TCP, e.src.into(), e.dst.into());
        if let Some(sock) = self.socks.write().get_mut(&key) {
            sock.closed = true;
        }
    }

    pub fn compact(&self) {
        self.socks.write().retain(|_, s| !s.closed);
    }
}
