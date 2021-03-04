use std::sync::Arc;
use anyhow::Result;
use log::debug;
use tokio::runtime::Runtime;
use kentik_api::{Client, Device};
use crate::capture::Flow;
use crate::sockets::{Event, Sockets};
use super::{get_or_create_device, pack, send};

pub struct Export {
    client: Arc<Client>,
    device: Arc<Device>,
    rt:     Runtime,
    socks:  Arc<Sockets>,
}

impl Export {
    pub fn new(client: Client, device: &str, plan: Option<u64>, socks: Arc<Sockets>) -> Result<Self> {
        let rt     = Runtime::new()?;
        let client = Arc::new(client);
        let device = rt.block_on(get_or_create_device(client.clone(), &device, plan))?;
        Ok(Self {
            client: client,
            device: Arc::new(device),
            rt:     rt,
            socks:  socks,
        })
    }

    pub fn export(&mut self, flows: Vec<Flow>, node: Option<Arc<String>>) -> Result<()> {
        debug!("exporting {} flows", flows.len());
        let rs = self.socks.merge(flows, node);

        for chunk in rs.chunks(16384) {
            let msg = pack(&self.device, chunk)?;
            let client = self.client.clone();
            let device = self.device.clone();
            self.rt.spawn(send(client, device, msg));
        }

        self.socks.compact();

        Ok(())
    }

    pub fn record(&mut self, e: Event) {
        self.socks.update(e);
    }
}
