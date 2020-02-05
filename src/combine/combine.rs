use std::collections::HashMap;
use std::mem;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use anyhow::Result;
use log::debug;
use parking_lot::Mutex;
use kentik_api::{Client, Device};
use crate::capture::flow::{Addr, Key};
use crate::collect::{Meta, Record};
use crate::export::{pack, send};
use crate::sockets::Process;

pub struct Combine {
    queue:   Mutex<HashMap<Key, Record>>,
    empty:   Mutex<HashMap<Key, Record>>,
    source:  Mutex<HashMap<Addr, Source>>,
    client:  Arc<Client>,
    device:  Arc<Device>,
    dump:    AtomicBool,
    timeout: Duration,
}

pub struct Source {
    node: Option<Arc<String>>,
    proc: Option<Arc<Process>>,
    seen: Instant,
}

impl Combine {
    pub fn new(client: Arc<Client>, device: Device) -> Self {
        Self {
            queue:   Mutex::new(HashMap::new()),
            empty:   Mutex::new(HashMap::new()),
            source:  Mutex::new(HashMap::new()),
            client:  client,
            device:  Arc::new(device),
            dump:    AtomicBool::new(false),
            timeout: Duration::from_secs(60),
        }
    }

    pub fn combine(&self, rs: Vec<Record>) {
        let mut queue  = self.queue.lock();
        let mut source = self.source.lock();

        let now = Instant::now();
        let def = || Source { node: None, proc: None, seen: now };

        let mut update = |addr: Addr, meta: &Meta| {
            let entry = source.entry(addr).or_insert_with(def);
            meta.node.as_ref().map(|node| entry.node = Some(node.clone()));
            meta.proc.as_ref().map(|proc| entry.proc = Some(proc.clone()));
            entry.seen = now;
        };

        for r in rs {
            update(r.flow.src, &r.src);
            update(r.flow.dst, &r.dst);
            queue.entry(r.flow.key()).and_modify(|entry| {
                entry.flow.bytes   += r.flow.bytes;
                entry.flow.packets += r.flow.packets;
            }).or_insert(r);
        }
    }

    pub fn export(&self) -> Result<()> {
        let mut queue  = self.queue.lock();
        let mut export = self.empty.lock();
        let mut source = self.source.lock();

        mem::swap(&mut *queue, &mut *export);
        drop(queue);

        let meta = |addr: &Addr| {
            source.get(addr).map(|s| {
                Meta {
                    node: s.node.clone(),
                    proc: s.proc.clone(),
                    ..Default::default()
                }
            }).unwrap_or_default()
        };

        for r in &mut export.values_mut() {
            r.src = meta(&r.flow.src);
            r.dst = meta(&r.flow.dst);
        }

        let now = Instant::now();
        source.retain(|_, s| now - s.seen < self.timeout);
        drop(source);

        if self.dump.load(Ordering::SeqCst) {
            debug!("combine state:");
            export.iter().for_each(print)
        }

        let rs  = export.drain().map(|(_, r)| r).collect();
        let msg = pack(&self.device, rs)?;

        let client = self.client.clone();
        let device = self.device.clone();
        tokio::spawn(send(client, device, msg));

        Ok(())
    }

    pub fn dump(&self) {
        let state = self.dump.load(Ordering::SeqCst);
        self.dump.store(!state, Ordering::SeqCst);
    }
}

fn print<'a>((key, rec): (&'a Key, &'a Record)) {
    let src = rec.src.proc.as_ref().map(|p| p.comm.as_str()).unwrap_or("??");
    let dst = rec.dst.proc.as_ref().map(|p| p.comm.as_str()).unwrap_or("??");
    debug!("{}:{} -> {}:{}: {} -> {}",
           key.1.addr, key.1.port,
           key.2.addr, key.2.port,
           src,        dst,
    );
}
