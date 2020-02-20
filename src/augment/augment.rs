use std::collections::HashMap;
use std::mem;
use std::net::IpAddr;
use std::sync::Arc;
use anyhow::Result;
use futures::prelude::*;
use log::{debug, error};
use parking_lot::Mutex;
use tokio::net::{TcpListener, TcpStream};
use tokio_serde::{SymmetricallyFramed, formats::SymmetricalJson};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};
use crate::collect::Record;
use crate::sockets::Process;
use super::object::{Object, Pod, Service, IP};

pub struct Augment {
    addr: String,
    kube: Mutex<HashMap<Key, Arc<Object>>>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum Key {
    CID(String),
    IP(IpAddr),
}

impl Augment {
    pub fn new(addr: String) -> Self {
        Self {
            addr: addr,
            kube: Mutex::new(HashMap::new()),
        }
    }

    pub fn merge(&self, rs: &mut [Record]) {
        let kube = self.kube.lock();

        let lookup = |ip: IpAddr, proc: &Option<Arc<Process>>| {
            kube.get(&Key::IP(ip)).or_else(|| {
                let cid = proc.as_ref()?.container.as_ref()?;
                kube.get(&Key::CID(cid.to_string()))
            }).cloned()
        };

        for r in rs {
            r.src.kube = lookup(r.flow.src.addr, &r.src.proc);
            r.dst.kube = lookup(r.flow.dst.addr, &r.dst.proc);
        }
    }

    pub fn update(&self, objs: Vec<Object>) {
        let mut map = HashMap::new();

        for o in objs.into_iter().map(Arc::new) {
            if let Object::Pod(Pod{ip, containers, ..}) = o.as_ref() {
                if let IP::Host(..) = ip {
                    for c in containers {
                        let key = Key::CID(c.id.clone());
                        let val = o.clone();
                        map.insert(key, val);
                    }
                } else if let IP::Pod(ip) = ip {
                    map.insert(Key::IP(*ip), o);
                }
            } else if let Object::Service(Service{ip, ..}) = o.as_ref() {
                map.insert(Key::IP(*ip), o);
            }
        }

        let mut kube = self.kube.lock();
        mem::swap(&mut *kube, &mut map);
    }

    pub async fn listen(self: Arc<Self>) {
        match listen(self.addr.clone(), self).await {
            Ok(()) => debug!("augment finished"),
            Err(e) => error!("augment failed: {}", e),
        }
    }
}


async fn listen(addr: String, augment: Arc<Augment>) -> Result<()> {
    let mut listener = TcpListener::bind(&addr).await?;
    loop {
        let (sock, addr) = listener.accept().await?;
        debug!("connection from {}", addr);
        let augment = augment.clone();

        tokio::spawn(async move {
            match client(sock, augment).await {
                Ok(()) => debug!("client {} finished", addr),
                Err(e) => error!("client {} error: {}", addr, e),
            }
        });
    }
}

async fn client(sock: TcpStream, augment: Arc<Augment>) -> Result<()> {
    let mut length = LengthDelimitedCodec::new();
    length.set_max_frame_length(32 * 1024 * 1024);
    let framed = FramedRead::new(sock, length);
    let format = SymmetricalJson::<Vec<crate::augment::object::Object>>::default();

    let mut codec = SymmetricallyFramed::new(framed, format);

    while let Some(objs) = codec.try_next().await? {
        augment.update(objs);
    }

    Ok(())
}
