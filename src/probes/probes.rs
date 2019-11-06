use std::mem::size_of;
use std::net::Ipv4Addr;
use std::os::raw::c_int;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use anyhow::Result;
use crossbeam_channel::{Sender, Receiver, bounded};
use ebpf::bpf::Program;
use ebpf::elf::{self, Map};
use ebpf::ffi::{bpf_map_create_arg};
use ebpf::ffi::bpf_map_type::BPF_MAP_TYPE_PERF_EVENT_ARRAY;
use log::{debug, error, warn};
use nixv::{Version, kernel};
use perf::map::Event;
use perf::sys::*;
use perf::ffi::*;
use super::Socket;
use super::version::LinuxVersionCode;
use super::events;
use super::poll::Poll;

pub struct Probes {
    programs: Vec<Program>,
}

impl Probes {
    pub fn load(code: &[u8], version: Option<Version>) -> Result<Self> {
        let cpus = num_cpus::get();

        let mut loader = elf::Loader::new(code)?;

        if let Some(system) = version.or_else(|| kernel::version()) {
            let code = Version::decode(loader.version);
            if code != system {
                warn!("eBPF code built for Linux {}", code);
                warn!("system kernel version: {}", system);
                loader.version = system.encode();
            }
        }

        let symbol = loader.symbols.iter().find(|s| s.name == "events").cloned();
        loader.maps.push(Map {
            symbol: symbol.unwrap(),
            create: bpf_map_create_arg {
                map_type:    BPF_MAP_TYPE_PERF_EVENT_ARRAY as u32,
                key_size:    size_of::<c_int>() as u32,
                val_size:    size_of::<c_int>() as u32,
                max_entries: cpus as u32,
                .. Default::default()
            }
        });

        let programs = loader.load()?;

        Ok(Self {
            programs: programs,
        })
    }

    pub fn start(&self, shutdown: Arc<AtomicBool>) -> Result<(Receiver<Socket>, JoinHandle<()>)> {
        let cpus = num_cpus::get();

        for prog in &self.programs {
            debug!("attaching {}", prog.name);
            let id = events::create(prog)?.unwrap();
            let _  = events::attach(id, prog.fd)?;
        }

        let map = self.programs.iter().flat_map(|prog| {
            prog.maps.iter().find(|map| map.name == "events")
        }).next().unwrap();

        let fds = (0..cpus).map(|n| {
            let mut attr = perf_event_attr::default();
            attr.type_        = PERF_TYPE_SOFTWARE;
            attr.config       = PERF_COUNT_SW_BPF_OUTPUT;
            attr.sample_type  = PERF_SAMPLE_RAW;
            attr.sample       = perf_event_sample_arg { sample_period: 1 };
            attr.wakeup       = perf_event_wakeup_arg { wakeup_events: 1 };

            let pid = -1;
            let cpu = n as i32;

            let fd = perf_event_open(&attr, pid, cpu, -1, 0)?;
            perf_event_ioc_enable(fd)?;
            map.insert(&cpu, &fd)?;

            Ok(fd)
        }).collect::<Result<Vec<c_int>>>()?;

        let (tx, rx) = bounded(1024);

        let task = || match poll(fds, tx, shutdown) {
            Ok(()) => (),
            Err(e) => error!("probe poll failed: {:?}", e),
        };

        Ok((rx, thread::spawn(task)))
    }
}

fn poll(fds: Vec<c_int>, tx: Sender<Socket>, shutdown: Arc<AtomicBool>) -> Result<()> {
    // TODO: munmap and close on drop?
    let mut maps = Vec::with_capacity(fds.len());
    let mut poll = Poll::new(&fds, 64).unwrap();

    while !shutdown.load(Ordering::Acquire) {
        let n = poll.poll(&mut maps, -1).unwrap();
        for map in &mut maps[..n] {
            #[repr(C)]
            #[derive(Debug)]
            struct Data {
                event: u32,
                pid:   u32,
                saddr: u32,
                sport: u32,
                daddr: u32,
                dport: u32,
            }

            let mut events = map.events::<Data>();
            while let Some(event) = events.next() {
                if let Event::Event(data) = event {
                    let pid = data.pid as u32;
                    let src = (Ipv4Addr::from(data.saddr.to_be()), data.sport as u16).into();
                    let dst = (Ipv4Addr::from(data.daddr.to_be()), data.dport as u16).into();

                    tx.send(match data.event {
                        1 => Socket::Connect(pid, src, dst),
                        2 => Socket::Accept(pid, src, dst),
                        3 => Socket::Close(pid, src, dst),
                        n => panic!("bad event type: {}", n),
                    })?;
                } else {
                    warn!("unhandled event {:?}", event);
                }
            }
        }
    }

    Ok(())
}
