use std::sync::Arc;
use tokio::sync::mpsc::{Sender, error::TrySendError};
use tokio::runtime::Handle;
use crate::capture::Flow;
use crate::sockets::Sockets;
use super::Record;

#[derive(Clone)]
pub struct Sink {
    node:   Option<Arc<String>>,
    socks:  Arc<Sockets>,
    tx:     Sender<Vec<Record>>,
    handle: Handle,
}

impl Sink {
    pub fn new(node: Option<Arc<String>>, socks: Arc<Sockets>, tx: Sender<Vec<Record>>, handle: Handle) -> Self {
        Self { node, socks, tx, handle }
    }

    pub async fn collect(&self, flows: Vec<Flow>) -> Result<(), TrySendError<Vec<Record>>> {
        let records = self.socks.merge(flows, self.node.clone());
        self.socks.compact();
        self.tx.try_send(records)
    }

    pub fn dispatch(&self, flows: Vec<Flow>) -> Result<(), TrySendError<Vec<Record>>> {
        self.handle.block_on(self.collect(flows))
    }
}
