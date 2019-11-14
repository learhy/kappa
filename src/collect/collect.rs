use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use futures::channel::mpsc::{Receiver, Sender, channel};
use log::warn;
use tokio::prelude::*;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::time::delay_for;
use tokio_serde::{self, formats::Json};
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};
use crate::capture::Flow;
use crate::sockets::Sockets;
use super::Record;

pub struct Collect {
    tx:    Sender<Vec<Record>>,
    socks: Arc<Sockets>,
}

impl Collect {
    pub fn new(agg: String, socks: Arc<Sockets>, rt: &Runtime) -> Self {
        let (tx, rx) = channel(1024);
        rt.spawn(dispatch(agg, rx));
        Self {
            tx:    tx,
            socks: socks,
        }
    }

    pub fn collect(&mut self, flows: Vec<Flow>) -> Result<()> {
        let records = self.socks.merge(flows);
        match self.tx.try_send(records) {
            Ok(()) => (),
            Err(e) => warn!("dispatch queue full: {:?}", e),
        };
        self.socks.compact();

        Ok(())
    }
}

async fn dispatch(agg: String, mut rx: Receiver<Vec<Record>>) {
    loop {
        let sock = connect(&agg).await;

        let length = LengthDelimitedCodec::new();
        let framed = FramedWrite::new(sock, length);
        let format = Json::default();

        let mut codec = tokio_serde::FramedWrite::new(framed, format);

        while let Some(recs) = rx.next().await {
            if let Err(e) = codec.send(recs).await {
                warn!("write error: {}", e);
                break;
            }
        }
    }
}

async fn connect(agg: &str) -> TcpStream {
    loop {
        let err = match TcpStream::connect(agg).await {
            Ok(sock) => return sock,
            Err(e)   => e,
        };

        warn!("connection error: {}", err);

        delay_for(Duration::from_secs(1)).await;
    }
}
