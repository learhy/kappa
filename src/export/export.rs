use std::sync::Arc;
use anyhow::Result;
use kentik_api::{Client, Device};
use crate::collect::Record;
use super::{get_or_create_device, pack, send};

pub struct Export {
    client: Arc<Client>,
    device: Arc<Device>,
}

impl Export {
    pub async fn new(client: Client, device: &str, plan: Option<u64>) -> Result<Self> {
        let client = Arc::new(client);
        let device = get_or_create_device(client.clone(), &device, plan).await?;
        Ok(Self {
            client: client,
            device: Arc::new(device),
        })
    }

    pub async fn export(&mut self, rs: Vec<Record>) -> Result<()> {
        for chunk in rs.chunks(16384) {
            let msg = pack::records(&self.device, chunk)?;
            let client = self.client.clone();
            let device = self.device.clone();
            send(client, device, msg).await;
        }
        Ok(())
    }
}
