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
use crate::collect::Record;
use crate::export::{pack, send};
use crate::sockets::Process;

pub struct Combine {
    queue:   Mutex<HashMap<Key, Record>>,
    empty:   Mutex<HashMap<Key, Record>>,
    procs:   Mutex<HashMap<Addr, Proc>>,
    client:  Arc<Client>,
    device:  Arc<Device>,
    dump:    AtomicBool,
    timeout: Duration,
}

pub struct Proc {
    proc: Arc<Process>,
    seen: Instant,
}

impl Combine {
    pub fn new(client: Arc<Client>, device: Device) -> Self {
        Self {
            queue:   Mutex::new(HashMap::new()),
            empty:   Mutex::new(HashMap::new()),
            procs:   Mutex::new(HashMap::new()),
            client:  client,
            device:  Arc::new(device),
            dump:    AtomicBool::new(false),
            timeout: Duration::from_secs(60),
        }
    }

    pub fn combine(&self, rs: Vec<Record>) {
        let mut queue = self.queue.lock();
        let mut procs = self.procs.lock();

        let now = Instant::now();

        let mut set = |addr: Addr, p: &Arc<Process>| {
            procs.insert(addr, Proc {
                proc: p.clone(),
                seen: now,
            });
        };

        for r in rs {
            r.src.as_ref().map(|p| set(r.flow.src, p));
            r.dst.as_ref().map(|p| set(r.flow.dst, p));
            queue.entry(r.flow.key()).and_modify(|entry| {
                entry.flow.bytes   += r.flow.bytes;
                entry.flow.packets += r.flow.packets;
            }).or_insert(r);
        }
    }

    pub fn export(&self) -> Result<()> {
        let mut queue  = self.queue.lock();
        let mut export = self.empty.lock();
        let mut procs  = self.procs.lock();

        mem::swap(&mut *queue, &mut *export);
        drop(queue);

        for r in &mut export.values_mut() {
            r.src = procs.get(&r.flow.src).map(|p| p.proc.clone());
            r.dst = procs.get(&r.flow.dst).map(|p| p.proc.clone());
        }

        if self.dump.load(Ordering::SeqCst) {
            debug!("combine state:");
            export.iter().for_each(print)
        }

        let rs  = export.drain().map(|(_, r)| r).collect();
        let msg = pack(&self.device, rs)?;

        let client = self.client.clone();
        let device = self.device.clone();
        tokio::spawn(send(client, device, msg));

        let now = Instant::now();
        procs.retain(|_, p| now - p.seen < self.timeout);

        Ok(())
    }

    pub fn dump(&self) {
        let state = self.dump.load(Ordering::SeqCst);
        self.dump.store(!state, Ordering::SeqCst);
    }
}

fn print<'a>((key, rec): (&'a Key, &'a Record)) {
    let src = rec.src.as_ref().map(|p| p.comm.as_str()).unwrap_or("??");
    let dst = rec.dst.as_ref().map(|p| p.comm.as_str()).unwrap_or("??");
    debug!("{}:{} -> {}:{}: {} -> {}",
           key.1.addr, key.1.port,
           key.2.addr, key.2.port,
           src,        dst,
    );
}
