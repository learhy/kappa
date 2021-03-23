use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use anyhow::Result;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{Sender, Receiver, channel};
use tokio::time::sleep;
use log::{debug, error};
use pcap::Device;
use pnet::datalink;
use pnet::util::MacAddr;
use super::{Add, Event};

pub struct Links {
    rx: Receiver<Event>,
}

impl Links {
    pub fn watch(handle: &Handle, _shutdown: Arc<AtomicBool>) -> Result<Self> {
        let (tx, rx) = channel(64);
        handle.spawn(async move {
            match monitor(tx).await {
                Ok(_)  => debug!("link monitor finished"),
                Err(e) => error!("link monitor failed: {:?}", e),
            }
        });
        Ok(Self { rx })
    }

    pub async fn recv(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

async fn monitor(tx: Sender<Event>) -> Result<()> {
    let mut links = HashSet::new();

    loop {
        let macs = datalink::interfaces().into_iter().map(|link| {
            (link.name, link.mac)
        }).collect::<HashMap<_, _>>();

        let curr = Device::list()?.into_iter().map(|d| {
            d.name
        }).collect::<HashSet<_>>();

        let copy = links.clone();

        for link in curr.difference(&copy) {
            let mac = macs.get(link).and_then(Option::clone);
            tx.send(add(link, mac)).await?;
            links.insert(link.to_string());
        }

        for link in copy.difference(&curr) {
            tx.send(Event::Delete(link.to_string())).await?;
            links.remove(link);
        }

        sleep(Duration::from_secs(60)).await;
    }
}

fn add(link: &str, mac: Option<MacAddr>) -> Event {
    Event::Add(Add {
        name:  link.to_string(),
        dev:   link.to_string(),
        mac:   mac,
        netns: None,
    })
}
