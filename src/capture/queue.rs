use std::collections::HashMap;
use anyhow::Result;
use crossbeam_channel::Sender;
use log::warn;
use pcap::Packet;
use pnet::util::MacAddr;
use time::Duration;
use super::{decode, Timestamp, timer::Timer};
use super::flow::{Flow, Key};
use crossbeam_channel::TrySendError::*;

pub struct Queue {
    queue:  HashMap<Key, Flow>,
    mac:    Option<MacAddr>,
    sample: u32,
    timer:  Timer,
    tx:     Sender<Vec<Flow>>,
    done:   bool,
}

impl Queue {
    pub fn new(mac: Option<MacAddr>, sample: u32, tx: Sender<Vec<Flow>>, interval: Duration) -> Self {
        Self {
            queue:  HashMap::new(),
            mac:    mac,
            sample: sample,
            timer:  Timer::new(interval),
            tx:     tx,
            done:   false,
        }
    }

    pub fn record(&mut self, pkt: Packet<'_>) -> Result<()> {
        if let Some(mut flow) = decode(self.mac, pkt) {
            flow.sample = self.sample;

            let ts  = flow.timestamp;
            let key = flow.key();

            self.queue.entry(key).and_modify(|entry| {
                entry.bytes   += flow.bytes;
                entry.packets += 1;
                entry.tos     |= flow.tos;
            }).or_insert(flow);

            self.export(ts);
        }
        Ok(())
    }

    pub fn export(&mut self, ts: Timestamp) {
        if self.timer.ready(ts) && self.queue.len() > 0 {
            let flows = self.queue.drain().map(|(_, flow)| {
                flow
            }).collect();

            match self.tx.try_send(flows) {
                Ok(_)                => (),
                Err(Full(_))         => warn!("capture channel full"),
                Err(Disconnected(_)) => self.done = true,
            }
        }
    }

    pub fn done(&self) -> bool {
        self.done
    }
}
