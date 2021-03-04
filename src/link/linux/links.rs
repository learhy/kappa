use anyhow::Result;
use nell::{Message, Netlink};
use nell::api::IFLA;
use nell::ffi::*;
use nell::sync::Socket;
use pnet::util::MacAddr;
use super::Peer;

#[derive(Debug, Default)]
pub struct Link {
    pub index: u32,
    pub name:  String,
    pub addr:  Option<MacAddr>,
    pub flags: u32,
    pub peer:  Option<Peer>,
}

pub fn links(sock: &mut Socket) -> Result<Vec<Link>> {
    let mut msg = Message::<rtgenmsg>::new(RTM_GETLINK);
    msg.set_flags(NLM_F_REQUEST | NLM_F_DUMP);
    msg.rtgen_family = AF_UNSPEC;
    sock.send(&msg)?;

    let mut links = Vec::new();

    while let Netlink::Msg(msg) = sock.recv::<ifinfomsg>()? {
        links.push(link(&msg)?);
    }

    Ok(links)
}

pub fn link(msg: &Message<ifinfomsg>) -> Result<Link> {
    let mut link = Link {
        index: msg.ifi_index as u32,
        flags: msg.ifi_flags,
        ..Link::default()
    };

    let mut index = None;
    let mut nsid  = None;

    for attr in msg.attrs() {
        match attr? {
            IFLA::IFName(name)    => link.name = name.to_string(),
            IFLA::Address(octets) => link.addr = mac(octets),
            IFLA::Link(link)      => index     = Some(link),
            IFLA::LinkNetNSID(id) => nsid      = Some(id),
            _                     => (),
        }
    }

    if let (Some(index), Some(nsid)) = (index, nsid) {
        link.peer = Some(Peer { index, nsid });
    }

    Ok(link)
}

fn mac(octets: &[u8]) -> Option<MacAddr> {
    match octets {
        &[a, b, c, d, e, f] => Some(MacAddr::new(a, b, c, d, e, f)),
        _                   => None,
    }
}
