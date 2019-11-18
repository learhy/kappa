use std::sync::Arc;
use anyhow::Result;
use log::{debug, warn};
use kentik_api::{Client, Device};
use kentik_api::Error::*;

pub async fn send(client: Arc<Client>, device: Arc<Device>, msg: Vec<u8>) {
    match client.flow(&device, msg).await {
        Ok(()) => (),
        Err(e) => warn!("failed to deliver flow: {:?}", e),
    }
}

pub async fn get_or_create_device(client: Arc<Client>, name: &str, plan: Option<u64>) -> Result<Device> {
    let device = match client.get_device_by_name(name).await {
        Ok(device)       => device,
        Err(App(_, 404)) => create_device(client, name, plan).await?,
        Err(e)           => Err(e)?,
    };
    debug!("device {:?}", device);
    Ok(device)
}

async fn create_device(client: Arc<Client>, name: &str, plan: Option<u64>) -> Result<Device> {
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

pub use export::Export;
pub use pack::pack;

mod export;
mod pack;
