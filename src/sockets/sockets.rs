use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use log::trace;
use parking_lot::Mutex;
use crate::capture::flow::{Flow, Key};
use crate::collect::Record;
use super::{Event, Kind, Process};

pub struct Sockets {
    socks:   Mutex<HashMap<Key, Socket>>,
    timeout: Duration,
}

#[derive(Debug)]
pub struct Socket {
    proc: Arc<Process>,
    srtt: Duration,
    seen: Instant,
}

impl Sockets {
    pub fn new() -> Self {
        Self {
            socks:   Mutex::new(HashMap::new()),
            timeout: Duration::from_secs(60),
        }
    }

    pub fn get(&self, key: &Key) -> Option<Arc<Process>> {
        self.socks.lock().get_mut(key).map(|s| {
            s.seen = Instant::now();
            s.proc.clone()
        })
    }

    pub fn merge(&self, flow: Vec<Flow>) -> Vec<Record> {
        let mut socks = self.socks.lock();

        let now  = Instant::now();
        let srtt = Mutex::new(Duration::from_micros(0));
        let mut lookup = |key: &Key| {
            socks.get_mut(key).map(|s| {
                s.seen = now;
                *srtt.lock() = s.srtt;
                s.proc.clone()
            })
        };

        flow.into_iter().map(|flow| {
            let src = lookup(&Key(flow.protocol, flow.src, flow.dst));
            let dst = lookup(&Key(flow.protocol, flow.dst, flow.src));
            Record {
                flow: flow,
                src:  src,
                dst:  dst,
                srtt: *srtt.lock(),
            }
        }).collect()
    }

    pub fn update(&self, e: Event) {
        match e.kind {
            Kind::Accept  => self.insert(e),
            Kind::Connect => self.insert(e),
            Kind::TX      => self.insert(e),
            Kind::RX      => self.insert(e),
            Kind::Close   => (),
        }
    }

    fn insert(&self, Event { kind, proto, src, dst, proc, srtt, .. }: Event) {
        let key = Key(proto, src.into(), dst.into());

        let new = || {
            trace!("{:?} {} -> {}: {} ({})", kind, src, dst, proc.comm, proc.pid);
            Socket {
                proc: Arc::new(proc),
                seen: Instant::now(),
                srtt: srtt,
            }
        };

        self.socks.lock().entry(key).and_modify(|sock| {
            sock.srtt = srtt;
        }).or_insert_with(new);
    }

    pub fn compact(&self) {
        let now = Instant::now();
        self.socks.lock().retain(|_, s| {
            now.saturating_duration_since(s.seen) < self.timeout
        });
    }
}
