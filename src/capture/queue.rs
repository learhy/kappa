use std::collections::HashMap;
use anyhow::Result;
use log::warn;
use pcap::Packet;
use pnet::util::MacAddr;
use time::Duration;
use crate::collect::Sink;
use super::{decode, Timestamp, timer::Timer};
use super::flow::{Flow, Key};
use tokio::sync::mpsc::error::TrySendError::*;

pub struct Queue {
    queue:  HashMap<Key, Flow>,
    mac:    Option<MacAddr>,
    sample: u32,
    timer:  Timer,
    sink:   Sink,
    done:   bool,
}

impl Queue {
    pub fn new(mac: Option<MacAddr>, sample: u32, sink: Sink, interval: Duration) -> Self {
        Self {
            queue:  HashMap::new(),
            mac:    mac,
            sample: sample,
            timer:  Timer::new(interval),
            sink:   sink,
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

            match self.sink.dispatch(flows) {
                Ok(())         => (),
                Err(Closed(_)) => self.done = true,
                Err(Full(_))   => warn!("dispatch queue full"),
            }
        }
    }

    pub fn done(&self) -> bool {
        self.done
    }
}
