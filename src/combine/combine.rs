use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::mem;
use anyhow::Result;
use log::debug;
use parking_lot::Mutex;
use kentik_api::{Client, Device};
use crate::capture::flow::Key;
use crate::collect::Record;
use crate::export::{pack, send};

pub struct Combine {
    queue:  Mutex<HashMap<Key, Record>>,
    empty:  Mutex<HashMap<Key, Record>>,
    client: Arc<Client>,
    device: Arc<Device>,
    dump:   AtomicBool,
}

impl Combine {
    pub fn new(client: Arc<Client>, device: Device) -> Self {
        Self {
            queue:  Mutex::new(HashMap::new()),
            empty:  Mutex::new(HashMap::new()),
            client: client,
            device: Arc::new(device),
            dump:   AtomicBool::new(false),
        }
    }

    pub fn combine(&self, rs: Vec<Record>) {
        let mut queue = self.queue.lock();
        for r in rs {
            queue.entry(r.flow.key()).and_modify(|entry| {
                entry.flow.bytes   += r.flow.bytes;
                entry.flow.packets += r.flow.packets;
                r.src.as_ref().map(|p| entry.src = Some(p.clone()));
                r.dst.as_ref().map(|p| entry.dst = Some(p.clone()));
            }).or_insert(r);
        }
    }

    pub fn export(&self) -> Result<()> {
        let mut queue  = self.queue.lock();
        let mut export = self.empty.lock();

        mem::swap(&mut *queue, &mut *export);
        drop(queue);

        if self.dump.swap(false, Ordering::SeqCst) {
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
        self.dump.store(true, Ordering::SeqCst);
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
