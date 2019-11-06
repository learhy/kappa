use std::env;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use anyhow::Result;
use crossbeam_channel::{Receiver, TryRecvError};
use nixv::Version;
use crate::probes::{self, Probes, Socket};
use super::{Event, Kind};
use TryRecvError::*;

pub struct Monitor {
    #[allow(unused)]
    probes: Probes,
    cache:  Cache,
    rx:     Receiver<Socket>,
}

impl Monitor {
    pub fn start(kernel: Option<Version>, shutdown: Arc<AtomicBool>) -> Result<Self> {
        let code    = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/bpf_kern.o"));
        let probes  = Probes::load(&code[..], kernel)?;
        let (rx, _) = probes.start(shutdown)?;

        let cache = Cache::new();

        // FIXME: remove
        probes::trace();

        Ok(Self { probes, cache, rx })
    }

    pub fn recv(&mut self) -> Result<Option<Event>> {
        match self.rx.try_recv() {
            Ok(socket)        => Ok(event(&mut self.cache, socket)),
            Err(Empty)        => Ok(None),
            Err(Disconnected) => Ok(None),
        }
    }
}

fn event(cache: &mut Cache, socket: Socket) -> Option<Event> {
    let (kind, pid, src, dst) = match socket {
        Socket::Connect(pid, src, dst) => (Kind::Connect, pid, src, dst),
        Socket::Accept(pid, src, dst)  => (Kind::Accept,  pid, src, dst),
        Socket::Close(pid, src, dst)   => (Kind::Close,   pid, src, dst),
    };

    let proc = cache.get(pid)?.clone();

    Some(Event {
        proc: proc,
        kind: kind,
        src:  src,
        dst:  dst,
    })
}

pub use cache::Cache;
pub use lookup::lookup;

mod cache;
mod lookup;
