use std::thread::{self, JoinHandle};
use std::time::{Duration as StdDuration};
use anyhow::{Result, anyhow};
use crossbeam_channel::Sender;
use log::{debug, info, error};
use pcap::{Capture, Active, Inactive, Device};
use pnet::util::MacAddr;
use pnet::datalink;
use time::Duration;
use super::queue::Queue;
use super::flow::{Flow, Timestamp};
use pcap::Error::*;

pub struct Source {
    cap:  Capture<Inactive>,
    mac:  Option<MacAddr>,
    name: String,
}

impl Source {
    pub fn new(cap: Capture<Inactive>, mac: Option<MacAddr>, name: String) -> Self {
        Self { cap, mac, name }
    }

    pub fn start(self, interval: StdDuration, tx: Sender<Vec<Flow>>) -> Result<JoinHandle<()>> {
        info!("starting capture on {}", self.name);

        let interval = Duration::from_std(interval)?;
        let queue    = Queue::new(self.mac, tx, interval);
        let cap      = self.cap.open()?;

        Ok(thread::spawn(move || match poll(cap, queue) {
            Ok(_)  => debug!("capture finished"),
            Err(e) => error!("capture failed: {:?}", e),
        }))
    }
}

fn poll(mut cap: Capture<Active>, mut queue: Queue) -> Result<()> {
    while !queue.done() {
        match cap.next() {
            Ok(packet)          => queue.record(packet)?,
            Err(TimeoutExpired) => queue.export(Timestamp::now()),
            Err(NoMorePackets)  => return Ok(()),
            Err(e)              => return Err(e)?,
        }
    }
    Ok(())
}

pub fn lookup(device: &str) -> Result<(Option<MacAddr>, Device)> {
    let interface = datalink::interfaces().into_iter().find(|i| {
        i.name == device
    }).ok_or_else(|| anyhow!("device {} not found", device))?;

    let device = Device::list()?.into_iter().find(|d| {
        d.name == device
    }).ok_or_else(|| anyhow!("device {} not found", device))?;

    Ok((interface.mac, device))
}
