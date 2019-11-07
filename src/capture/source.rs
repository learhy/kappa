use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use anyhow::Result;
use crossbeam_channel::Sender;
use log::{debug, info, warn};
use parking_lot::Mutex;
use pcap::{Capture, Active};
use super::config::{capture, Config};
use super::queue::Queue;
use super::flow::{Flow, Timestamp};
use pnet::util::MacAddr;
use pcap::Error::*;

pub struct Sources {
    pub cfg: Config,
    pub tx:  Sender<Vec<Flow>>,
    pub map: Arc<Mutex<HashMap<String, Source>>>,
}

#[derive(Debug)]
pub struct Source {
    stop: Arc<AtomicBool>,
}

impl Sources {
    pub fn new(cfg: Config, tx: Sender<Vec<Flow>>) -> Self {
        let map = Arc::new(Mutex::new(HashMap::new()));
        Self { cfg, tx, map }
    }

    pub fn add(&mut self, link: String, mac: Option<MacAddr>) -> Result<()> {
        if !self.check(&link) {
            return Ok(());
        }

        let cap = match capture(&link, &self.cfg)? {
            Some(cap) => cap,
            None      => return Ok(()),
        };

        let interval = time::Duration::from_std(self.cfg.interval)?;
        let queue    = Queue::new(mac, self.tx.clone(), interval);
        let mut task = Task::new(cap, queue);

        let source = Source { stop: task.stop.clone() };
        let map    = self.map.clone();
        self.map.lock().insert(link.clone(), source);

        info!("starting capture on {}", link);

        thread::spawn(move || {
            match task.poll() {
                Ok(()) => debug!("capture on {} finished", link),
                Err(e) => warn!("capture on {} stopped: {:?}", link, e),
            };
            map.lock().remove(&link);
        });


        Ok(())
    }

    pub fn del(&mut self, link: String) {
        if let Some(s) = self.map.lock().get(&link) {
            s.stop.store(true, Ordering::Release);
        }
    }

    fn check(&self, link: &str) -> bool {
        if !self.cfg.capture.is_match(link) {
            info!("link {} ignored", link);
            return false;
        }

        if self.cfg.exclude.is_match(link) {
            info!("link {} excluded", link);
            return false;
        }

        if self.map.lock().contains_key(link) {
            info!("link {} already active", link);
            return false;
        }

        true
    }
}

struct Task {
    cap:   Capture<Active>,
    queue: Queue,
    stop:  Arc<AtomicBool>,
}

impl Task {
    fn new(cap: Capture<Active>, queue: Queue) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        Self { cap, queue, stop }
    }

    fn poll(&mut self) -> Result<()> {
        while !self.stop.load(Ordering::Acquire) && !self.queue.done() {
            match self.cap.next() {
                Ok(packet)          => self.queue.record(packet)?,
                Err(TimeoutExpired) => self.queue.export(Timestamp::now()),
                Err(NoMorePackets)  => break,
                Err(e)              => return Err(e.into()),
            }
        }
        Ok(())
    }
}
