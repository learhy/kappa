use std::collections::HashMap;
use std::net::SocketAddr;
use crate::process::{Event, Process, Kind};

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Key(SocketAddr, SocketAddr);

#[derive(Debug)]
pub struct Entry {
    proc:   Process,
    closed: bool,
}

pub struct Sockets {
    socks: HashMap<Key, Entry>,
}

impl Sockets {
    pub fn new() -> Self {
        Self {
            socks: HashMap::new(),
        }
    }

    pub fn lookup(&self, (src, dst): (SocketAddr, SocketAddr)) -> Option<&Process> {
        self.socks.get(&Key(src, dst)).map(|e| &e.proc)
    }

    pub fn update(&mut self, e: Event) {
        let key = Key(e.src, e.dst);
        match e.kind {
            Kind::Accept  => self.insert(key, e.proc),
            Kind::Connect => self.insert(key, e.proc),
            Kind::Close   => self.finish(key),
        }
    }

    fn insert(&mut self, key: Key, proc: Process) {
        self.socks.insert(key, Entry {
            proc:   proc,
            closed: false,
        });
    }

    fn finish(&mut self, key: Key) {
        if let Some(entry) = self.socks.get_mut(&key) {
            entry.closed = true;
        }
    }

    pub fn compact(&mut self) {
        self.socks.retain(|_, s| !s.closed);
    }
}
