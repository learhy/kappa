use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use futures_util::sink::SinkExt;
use log::{debug, warn};
use tokio::net::TcpStream;
use tokio::runtime::Handle;
use tokio::time::sleep;
use tokio_serde::{SymmetricallyFramed, formats::SymmetricalJson};
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};
use crate::agg::Request;
use crate::sockets::Sockets;
use super::{Record, Sink};

pub struct Collect {
    node:   Arc<String>,
    tx:     Sender<Request>,
    socks:  Arc<Sockets>,
    dump:   Arc<AtomicBool>,
    handle: Handle,
}

impl Collect {
    pub fn new(agg: String, socks: Arc<Sockets>, handle: Handle, node: String) -> Self {
        let dump = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel(1024);
        handle.spawn(dispatch(agg, rx, dump.clone()));
        Self {
            node:   Arc::new(node),
            tx:     tx,
            socks:  socks,
            dump:   dump,
            handle: handle,
        }
    }

    pub fn sink(&self) -> Sink {
        let node   = self.node.clone();
        let socks  = self.socks.clone();
        let tx     = self.tx.clone();
        let handle = self.handle.clone();
        Sink::new(node, socks, tx, handle)
    }

    pub fn dump(&self) -> Arc<AtomicBool> {
        self.dump.clone()
    }
}

async fn dispatch(agg: String, mut rx: Receiver<Request>, dump: Arc<AtomicBool>) {
    loop {
        let sock = connect(&agg).await;

        let mut length = LengthDelimitedCodec::new();
        length.set_max_frame_length(32 * 1024 * 1024);
        let framed = FramedWrite::new(sock, length);
        let format = SymmetricalJson::default();

        let mut codec = SymmetricallyFramed::new(framed, format);

        while let Some(req) = rx.recv().await {
            if dump.load(Ordering::SeqCst) {
                if let Request::Traffic(rs) = &req {
                    debug!("collect state:");
                    rs.iter().for_each(print)
                }
            }

            if let Err(e) = codec.send(req).await {
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
