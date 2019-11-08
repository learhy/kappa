use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use anyhow::Result;
use crossbeam_channel::{Sender, Receiver, TryRecvError, unbounded};
use log::{debug, error};
use nell::{Family, Message, Netlink};
use nell::api::{Any, IFLA};
use nell::ffi::*;
use nell::sync::Socket;
use pnet::util::MacAddr;
use super::Event;
use TryRecvError::*;

const IFF_UP:      u32 = nell::ffi::IFF_UP      as u32;
const IFF_PROMISC: u32 = nell::ffi::IFF_PROMISC as u32;

pub struct Links {
    rx: Receiver<Event>,
}

impl Links {
    pub fn watch(shutdown: Arc<AtomicBool>) -> Result<Self> {
        let (tx, rx) = unbounded();
        thread::spawn(move || match monitor(tx, shutdown) {
            Ok(_)  => debug!("link monitor finished"),
            Err(e) => error!("link monitor failed: {:?}", e),
        });
        Ok(Self { rx })
    }

    pub fn recv(&mut self) -> Result<Option<Event>> {
        match self.rx.try_recv() {
            Ok(event)         => Ok(Some(event)),
            Err(Empty)        => Ok(None),
            Err(Disconnected) => Ok(None),
        }
    }
}

#[derive(Debug, Default)]
struct Link {
    name:  String,
    addr:  Option<MacAddr>,
    flags: u32,
}

fn monitor(tx: Sender<Event>, shutdown: Arc<AtomicBool>) -> Result<()> {
    let mut sock = Socket::new(Family::Route)?;

    let mut msg = Message::<rtgenmsg>::new(RTM_GETLINK);
    msg.set_flags(NLM_F_REQUEST | NLM_F_DUMP);
    msg.rtgen_family = AF_UNSPEC;
    sock.send(&msg)?;

    while let Netlink::Msg(msg) = sock.recv::<ifinfomsg>()? {
        let link = link(msg)?;
        if link.flags & IFF_UP > 0 {
            tx.send(Event::Add(link.name, link.addr))?;
        }
    }

    while !shutdown.load(Ordering::Acquire) {
        let mut sock = Socket::new(Family::Route)?;
        sock.bind(0, RTMGRP_LINK)?;

        while let Netlink::Msg(msg) = sock.recv::<Any>()? {
            if shutdown.load(Ordering::Acquire) {
                break;
            }

            if let Any::IFInfo(msg) = msg.any() {
                let link = link(msg)?;
                let up = link.flags & IFF_UP > 0 && msg.ifi_change & IFF_PROMISC == 0;
                match msg.nlmsg_type() {
                    RTM_NEWLINK if up => tx.send(Event::Add(link.name, link.addr))?,
                    RTM_DELLINK       => tx.send(Event::Del(link.name))?,
                    _                 => ()
                }
            }
        }
    }

    Ok(())
}

fn link(msg: &Message<ifinfomsg>) -> Result<Link> {
    let mut link = Link {
        flags: msg.ifi_flags,
        ..Link::default()
    };

    for attr in msg.attrs() {
        match attr? {
            IFLA::IFName(name)    => link.name = name.to_string(),
            IFLA::Address(octets) => link.addr = mac(octets),
            _                     => (),
        }
    }

    Ok(link)
}

fn mac(octets: &[u8]) -> Option<MacAddr> {
    match octets {
        &[a, b, c, d, e, f] => Some(MacAddr::new(a, b, c, d, e, f)),
        _                   => None,
    }
}
