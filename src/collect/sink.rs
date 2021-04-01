use std::sync::Arc;
use tokio::sync::mpsc::{Sender, error::TrySendError};
use tokio::runtime::Handle;
use crate::agg::Request;
use crate::capture::Flow;
use crate::sockets::Sockets;

#[derive(Clone)]
pub struct Sink {
    node:   Arc<String>,
    socks:  Arc<Sockets>,
    tx:     Sender<Request>,
    handle: Handle,
}

impl Sink {
    pub fn new(node: Arc<String>, socks: Arc<Sockets>, tx: Sender<Request>, handle: Handle) -> Self {
        Self { node, socks, tx, handle }
    }

    pub async fn collect(&self, flows: Vec<Flow>) -> Result<(), TrySendError<Request>> {
        let records = self.socks.merge(flows, self.node.clone());
        self.socks.compact();
        self.export(Request::Traffic(records)).await
    }

    pub fn dispatch(&self, flows: Vec<Flow>) -> Result<(), TrySendError<Request>> {
        self.handle.block_on(self.collect(flows))
    }

    pub async fn export(&self, req: Request) -> Result<(), TrySendError<Request>> {
        self.tx.try_send(req)
    }
}
