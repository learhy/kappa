use std::convert::TryFrom;
use std::env;
use std::net::Ipv4Addr;
use std::os::raw::c_int;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use anyhow::Result;
use log::{debug, error, warn};
use nixv::Version;
use crate::probes::{self, Probes, Poll};
use crate::sockets::Sockets;
use super::{Event, Kind};
use super::cache::Cache;

static BYTECODE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/bpf_kern.o"));

pub struct Procs {
    #[allow(unused)]
    probes: Probes,
    socks:  Arc<Sockets>,
}

impl Procs {
    pub fn watch(kernel: Option<Version>, code: Option<Vec<u8>>, shutdown: Arc<AtomicBool>) -> Result<Self> {
        let code   = code.as_ref().map(Vec::as_slice).unwrap_or(&BYTECODE);
        let mut probes = Probes::load(&code[..], kernel)?;
        let fds    = probes.open()?;
        let socks  = Arc::new(Sockets::new());

        let socks2 = socks.clone();
        thread::spawn(move || match monitor(fds, socks2, shutdown) {
            Ok(_)  => debug!("sock monitor finished"),
            Err(e) => error!("sock monitor failed: {:?}", e),
        });

        // FIXME: remove
        probes::trace();

        Ok(Self { probes, socks })
    }

    pub fn sockets(&self) -> Arc<Sockets> {
        self.socks.clone()
    }
}

#[repr(C)]
#[derive(Debug)]
struct Data {
    event: u32,
    pid:   u32,
    proto: u32,
    saddr: u32,
    sport: u32,
    daddr: u32,
    dport: u32,
    srtt:  u32,
}

fn monitor(fds: Vec<c_int>, socks: Arc<Sockets>, shutdown: Arc<AtomicBool>) -> Result<()> {
    // TODO: munmap and close on drop?
    let mut maps  = Vec::with_capacity(fds.len());
    let mut poll  = Poll::new(&fds, 64).unwrap();
    let mut cache = Cache::new();

    while !shutdown.load(Ordering::Acquire) {
        let n = poll.poll(&mut maps, -1).unwrap();
        for map in &mut maps[..n] {
            let mut events = map.events::<Data>();
            while let Some(event) = events.next() {
                if let perf::map::Event::Event(data) = event {
                    if let Some(event) = resolve(data, &mut cache) {
                        socks.update(event);
                    }
                } else {
                    warn!("unhandled event {:?}", event);
                }
            }
        }
    }

    Ok(())
}

fn resolve(data: &Data, cache: &mut Cache) -> Option<Event> {
    let &Data { pid, saddr, daddr, srtt, .. } = data;

    let proto = u16::try_from(data.proto).ok()?;
    let sport = u16::try_from(data.sport).ok()?;
    let dport = u16::try_from(data.dport).ok()?;
    let src   = (Ipv4Addr::from(saddr.to_be()), sport).into();
    let dst   = (Ipv4Addr::from(daddr.to_be()), dport).into();

    let kind = match data.event {
        1 => Kind::Connect,
        2 => Kind::Accept,
        3 => Kind::TX,
        4 => Kind::RX,
        5 => Kind::Close,
        _ => return None,
    };

    Some(Event {
        kind:  kind,
        proto: proto.into(),
        src:   src,
        dst:   dst,
        srtt:  Duration::from_micros(srtt as u64),
        proc:  cache.get(pid)?.clone(),
    })
}
