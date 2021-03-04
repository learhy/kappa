use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use anyhow::Result;
use futures::channel::mpsc::{Receiver, Sender, channel};
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use log::{debug, warn};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::time::sleep;
use tokio_serde::{SymmetricallyFramed, formats::SymmetricalJson};
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};
use crate::capture::Flow;
use crate::sockets::Sockets;
use super::Record;

pub struct Collect {
    node:  Option<Arc<String>>,
    tx:    Sender<Vec<Record>>,
    socks: Arc<Sockets>,
    dump:  Arc<AtomicBool>,
}

impl Collect {
    pub fn new(agg: String, socks: Arc<Sockets>, rt: &Runtime, node: Option<String>) -> Self {
        let dump = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel(1024);
        rt.spawn(dispatch(agg, rx, dump.clone()));
        Self {
            node:  node.map(Arc::new),
            tx:    tx,
            socks: socks,
            dump:  dump,
        }
    }

    pub fn collect(&mut self, flows: Vec<Flow>) -> Result<()> {
        let records = self.socks.merge(flows, self.node.clone());
        match self.tx.try_send(records) {
            Ok(()) => (),
            Err(e) => warn!("dispatch queue full: {:?}", e),
        };
        self.socks.compact();

        Ok(())
    }

    pub fn dump(&self) -> Arc<AtomicBool> {
        self.dump.clone()
    }
}

async fn dispatch(agg: String, mut rx: Receiver<Vec<Record>>, dump: Arc<AtomicBool>) {
    loop {
        let sock = connect(&agg).await;

        let mut length = LengthDelimitedCodec::new();
        length.set_max_frame_length(32 * 1024 * 1024);
        let framed = FramedWrite::new(sock, length);
        let format = SymmetricalJson::default();

        let mut codec = SymmetricallyFramed::new(framed, format);

        while let Some(recs) = rx.next().await {
            if dump.load(Ordering::SeqCst) {
                debug!("collect state:");
                recs.iter().for_each(print)
            }

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

        sleep(Duration::from_secs(1)).await;
    }
}

fn print(rec: &Record) {
    let src = rec.src.proc.as_ref().map(|p| p.comm.as_str()).unwrap_or("??");
    let dst = rec.dst.proc.as_ref().map(|p| p.comm.as_str()).unwrap_or("??");
    debug!("{}:{} -> {}:{}: {} -> {}",
           rec.flow.src.addr, rec.flow.src.port,
           rec.flow.dst.addr, rec.flow.dst.port,
           src,               dst,
    );
}
