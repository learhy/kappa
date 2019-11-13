use std::sync::Arc;
use anyhow::Result;
use log::{debug, warn};
use tokio::runtime::Runtime;
use kentik_api::{Client, Device};
use kentik_api::Error::*;
use crate::capture::Flow;
use crate::sockets::{Event, Sockets};
use super::pack;

pub struct Export {
    client: Client,
    device: Device,
    rt:     Runtime,
    socks:  Arc<Sockets>,
}

impl Export {
    pub fn new(client: Client, device: &str, plan: Option<u64>, socks: Arc<Sockets>) -> Result<Self> {
        let rt     = Runtime::new()?;
        let device = rt.block_on(get_or_create_device(&client, &device, plan))?;
        Ok(Self {
            client: client,
            device: device,
            rt:     rt,
            socks:  socks,
        })
    }

    pub fn export(&mut self, flows: Vec<Flow>) -> Result<()> {
        debug!("exporting {} flows", flows.len());
        let msg = pack(&self.device, &*self.socks, flows)?;

        // FIXME: don't want these clones
        let client = self.client.clone();
        let device = self.device.clone();
        self.rt.spawn(send(client, device, msg));

        self.socks.compact();

        Ok(())
    }

    pub fn record(&mut self, e: Event) {
        self.socks.update(e);
    }
}

async fn send(client: Client, device: Device, msg: Vec<u8>) {
    match client.flow(&device, msg).await {
        Ok(()) => (),
        Err(e) => warn!("failed to deliver flow: {:?}", e),
    }
}

async fn get_or_create_device(client: &Client, name: &str, plan: Option<u64>) -> Result<Device> {
    let device = match client.get_device_by_name(name).await {
        Ok(device)       => device,
        Err(App(_, 404)) => create_device(client, name, plan).await?,
        Err(e)           => Err(e)?,
    };
    debug!("device {:?}", device);
    Ok(device)
}

async fn create_device(client: &Client, name: &str, plan: Option<u64>) -> Result<Device> {
    debug!("creating device {}", name);
    Ok(client.create_device(Device {
        name:        name.to_owned(),
        kind:        "host-nprobe-dns-www".to_owned(),
        subtype:     "kappa".to_owned(),
        bgp_type:    "none".to_owned(),
        cdn_attr:    "N".to_owned(),
        sample_rate: 1,
        plan_id:     plan,
        ..Default::default()
    }).await?)
}
