use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use anyhow::Result;
use crossbeam_channel::{Sender, Receiver, TryRecvError, unbounded};
use log::{debug, error};
use nell::{Family, Netlink};
use nell::api::Any;
use nell::ffi::*;
use nell::sync::Socket;
use crate::link::{Add, Event};
use super::Link;
use super::{link, links, peer};
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

fn monitor(tx: Sender<Event>, shutdown: Arc<AtomicBool>) -> Result<()> {
    let mut sock = Socket::new(Family::ROUTE)?;

    for link in links(&mut sock)? {
        if link.flags & IFF_UP > 0 {
            tx.send(add(link))?;
        }
    }

    while !shutdown.load(Ordering::Acquire) {
        let mut sock = Socket::new(Family::ROUTE)?;
        sock.bind(0, RTMGRP_LINK)?;

        while let Netlink::Msg(msg) = sock.recv::<()>()? {
            if shutdown.load(Ordering::Acquire) {
                break;
            }

            if let Any::IFInfo(msg) = msg.any() {
                let link = link(&msg)?;
                let up = link.flags & IFF_UP > 0 && msg.ifi_change & IFF_PROMISC == 0;
                match msg.nlmsg_type() {
                    RTM_NEWLINK if up => tx.send(add(link))?,
                    RTM_DELLINK       => tx.send(del(link))?,
                    _                 => ()
                }
            }
        }
    }

    Ok(())
}

fn add(link: Link) -> Event {
    let name = link.name.clone();
    let peer = link.peer.map(peer);
    let (dev, mac, netns) = match peer.transpose() {
        Ok(Some((netns, link))) => (link.name, link.addr, Some(netns)),
        Ok(None)                => (link.name, link.addr, None),
        Err(e)                  => return Event::Error(name, e.into()),
    };
    Event::Add(Add { name, dev, mac, netns })
}

fn del(Link { name, .. }: Link) -> Event {
    Event::Delete(name)
}
