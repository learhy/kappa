use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use anyhow::Result;
use crossbeam_channel::{Sender, Receiver, TryRecvError, unbounded};
use log::{debug, error};
use pcap::Device;
use pnet::datalink;
use super::Event;
use TryRecvError::*;

pub struct Links {
    rx: Receiver<Event>,
}

impl Links {
    pub fn watch(shutdown: Arc<AtomicBool>) -> Result<Self> {
        let (tx, rx) = unbounded();
        thread::spawn(move || match monitor(tx, shutdown) {
            Ok(_)  => debug!("link monitor finished"),
            Err(e) => error!("link monitor failed: {:?}", e),
        });
        Ok(Self { rx })
    }

    pub fn recv(&mut self) -> Result<Option<Event>> {
        match self.rx.try_recv() {
            Ok(event)         => Ok(Some(event)),
            Err(Empty)        => Ok(None),
            Err(Disconnected) => Ok(None),
        }
    }
}

fn monitor(tx: Sender<Event>, shutdown: Arc<AtomicBool>) -> Result<()> {
    let mut links = HashSet::new();

    while !shutdown.load(Ordering::Acquire) {
        let macs = datalink::interfaces().into_iter().map(|link| {
            (link.name, link.mac)
        }).collect::<HashMap<_, _>>();

        let curr = Device::list()?.into_iter().map(|d| {
            d.name
        }).collect::<HashSet<_>>();

        let copy = links.clone();

        for link in curr.difference(&copy) {
            let mac = macs.get(link).and_then(Option::clone);
            tx.send(Event::Add(link.to_string(), mac))?;
            links.insert(link.to_string());
        }

        for link in copy.difference(&curr) {
            tx.send(Event::Del(link.to_string()))?;
            links.remove(link);
        }

        thread::sleep(Duration::from_secs(60));
    }
    Ok(())
}
