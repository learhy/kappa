use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use anyhow::Result;
use log::{log_enabled, trace, Level::Trace};
use nixv::Version;
use parking_lot::Mutex;
use crate::capture::flow::{Flow, Protocol};
use crate::collect::{Meta, Record};
use crate::process::{Process, Procs};
use super::{Event, Kind, Monitor};

pub struct Sockets {
    kernel:  Mutex<Option<Version>>,
    code:    Mutex<Option<Vec<u8>>>,
    procs:   Procs,
    socks:   Mutex<HashMap<Key, Socket>>,
    timeout: Duration,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Key(Protocol, IpAddr, u16);

#[derive(Debug)]
pub struct Socket {
    proc: Option<Arc<Process>>,
    srtt: Duration,
    seen: Instant,
}

impl Sockets {
    pub fn new(procs: Procs, kernel: Option<Version>, code: Option<Vec<u8>>) -> Result<Self> {
        Ok(Self {
            kernel:  Mutex::new(kernel),
            code:    Mutex::new(code),
            procs:   procs,
            socks:   Mutex::new(HashMap::new()),
            timeout: Duration::from_secs(60),
        })
    }

    pub fn watch(self: Arc<Self>, shutdown: Arc<AtomicBool>) -> Result<Monitor> {
        let kernel = self.kernel.lock().take();
        let code   = self.code.lock().take();

        let mut monitor = Monitor::new(kernel, code)?;
        monitor.watch(self.clone(), shutdown)?;

        Ok(monitor)
    }

    pub fn get(&self, key: &Key) -> Option<Arc<Process>> {
        self.socks.lock().get_mut(key).and_then(|s| {
            s.seen = Instant::now();
            s.proc.clone()
        })
    }

    pub fn merge(&self, flow: Vec<Flow>, node: Arc<String>) -> Vec<Record> {
        let mut socks = self.socks.lock();

        let now  = Instant::now();
        let srtt = Mutex::new(Duration::from_micros(0));

        let mut meta = |key: &Key| {
            let proc = socks.get_mut(key).and_then(|s| {
                s.seen = now;
                *srtt.lock() = s.srtt;
                s.proc.clone()
            });

            Meta {
                proc: proc,
                node: Some(node.clone()),
                ..Default::default()
            }
        };

        flow.into_iter().map(|flow| {
            let src = meta(&Key(flow.protocol, flow.src.addr, flow.src.port));
            let dst = meta(&Key(flow.protocol, flow.dst.addr, flow.dst.port));
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

    fn insert(&self, Event { kind, pid, proto, src, dst, srtt, .. }: Event) {
        let key = Key(proto, src.ip(), src.port());

        let new = || {
            let proc = self.procs.get(pid);

            if log_enabled!(Trace) {
                if let Some(proc) = &proc {
                    let comm = &proc.comm;
                    let pid  = proc.pid;
                    trace!("{:?} {} -> {}: {} ({})", kind, src, dst, comm, pid);
                }
            }

            Socket {
                proc: proc,
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
