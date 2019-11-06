use std::collections::HashMap;
use anyhow::Result;
use crossbeam_channel::Sender;
use log::warn;
use pcap::Packet;
use pnet::util::MacAddr;
use time::Duration;
use super::{decode, timer::Timer};
use super::flow::{Flow, Key, Timestamp};
use crossbeam_channel::TrySendError::*;

pub struct Queue {
    queue: HashMap<Key, Flow>,
    mac:   Option<MacAddr>,
    tx:    Sender<Vec<Flow>>,
    timer: Timer,
    done:  bool,
}

impl Queue {
    pub fn new(mac: Option<MacAddr>, tx: Sender<Vec<Flow>>, interval: Duration) -> Self {
        Self {
            queue: HashMap::new(),
            mac:   mac,
            tx:    tx,
            timer: Timer::new(interval),
            done:  false,
        }
    }

    pub fn record(&mut self, pkt: Packet<'_>) -> Result<()> {
        if let Some(flow) = decode(self.mac, pkt) {
            let ts    = flow.timestamp;
            let tos   = flow.tos;
            let bytes = flow.bytes;
            let key   = flow.key();

            let entry = self.queue.entry(key).or_insert(flow);

            // FIXME: this double counts the first time a flow is inserted
            entry.bytes    += bytes;
            entry.packets  += 1;
            entry.tos      |= tos;

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
