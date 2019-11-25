use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use anyhow::Result;
use crossbeam_channel::Sender;
use log::{debug, info, warn};
use parking_lot::Mutex;
use crate::link::Add;
use crate::os::setns;
use super::{capture, Config, Sample, Timestamp};
use super::queue::Queue;
use super::flow::Flow;
use pcap::Error::*;

pub struct Sources {
    pub cfg: Arc<Config>,
    pub tx:  Sender<Vec<Flow>>,
    pub map: Arc<Mutex<HashMap<String, Source>>>,
}

#[derive(Debug)]
pub struct Source {
    stop: Arc<AtomicBool>,
}

impl Sources {
    pub fn new(cfg: Config, tx: Sender<Vec<Flow>>) -> Self {
        let map = Mutex::new(HashMap::new());
        Self {
            cfg: Arc::new(cfg),
            tx:  tx,
            map: Arc::new(map),
        }
    }

    pub fn add(&mut self, Add { name, dev, mac, netns }: Add) -> Result<()> {
        let name = match &netns {
            Some(_) => format!("{}-{}", name, dev),
            None    => name,
        };

        if !self.check(&name) {
            return Ok(());
        }

        let interval = time::Duration::from_std(self.cfg.interval)?;
        let sample   = match self.cfg.sample {
            Sample::Rate(n) => n,
            Sample::None    => 1,
        };

        let sender = self.tx.clone();
        let queue  = Queue::new(mac, sample, sender, interval);
        let stop   = Arc::new(AtomicBool::new(false));

        let source = Source { stop: stop.clone() };
        let cfg    = self.cfg.clone();
        let map    = self.map.clone();
        self.map.lock().insert(name.clone(), source);

        let mut task = Task::new(cfg, queue, stop);

        thread::spawn(move || {
            info!("starting {} capture", name);
            match task.poll(&name, dev, netns) {
                Ok(()) => debug!("capture {} finished", name),
                Err(e) => warn!("capture {} stopped: {:?}", name, e),
            };
            map.lock().remove(&name);
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
    cfg:   Arc<Config>,
    queue: Queue,
    stop:  Arc<AtomicBool>,
}

impl Task {
    fn new(cfg: Arc<Config>,queue: Queue, stop: Arc<AtomicBool>) -> Self {
        Self { cfg, queue, stop }
    }

    fn poll(&mut self, name: &str, dev: String, netns: Option<File>) -> Result<()> {
        if let Some(ns) = netns {
            setns(&ns)?;
        }

        let mut cap = capture(name, &dev, &self.cfg)?;

        while !self.stop.load(Ordering::Acquire) && !self.queue.done() {
            match cap.next() {
                Ok(packet)          => self.queue.record(packet)?,
                Err(TimeoutExpired) => self.queue.export(Timestamp::now()),
                Err(NoMorePackets)  => break,
                Err(e)              => return Err(e.into()),
            }
        }
        Ok(())
    }
}
