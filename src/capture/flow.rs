use std::fmt;
use std::mem;
use std::net::{IpAddr, SocketAddr};
use pnet::packet::tcp::TcpPacket;
use pnet::util::MacAddr;
use serde::{Serialize, Deserialize};
use super::Timestamp;

pub const FIN: u16 = 0b00001;
pub const SYN: u16 = 0b00010;
pub const RST: u16 = 0b00100;
pub const ACK: u16 = 0b10000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flow {
    pub timestamp: Timestamp,
    pub ethernet:  Ethernet,
    pub protocol:  Protocol,
    pub src:       Addr,
    pub dst:       Addr,
    pub tos:       u8,
    pub transport: Transport,
    pub packets:   usize,
    pub fragments: u16,
    pub bytes:     usize,
    pub sample:    u32,
    pub direction: Direction,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Ethernet {
    pub src:  MacAddr,
    pub dst:  MacAddr,
    pub vlan: Option<u16>
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum Protocol {
    ICMP,
    TCP,
    UDP,
    Other(u16),
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Addr {
    pub addr: IpAddr,
    pub port: u16,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Transport {
    ICMP,
    TCP  { seq: u32, flags: u16, window: Window },
    UDP,
    Other,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Window {
    pub size:  u32,
    pub scale: u8,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Key(pub Protocol, pub Addr, pub Addr);

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Direction {
    In, Out, Unknown
}

impl Flow {
    pub fn key(&self) -> Key {
        Key(self.protocol, self.src, self.dst)
    }

    pub fn tcp_flags(&self) -> u16 {
        match self.transport {
            Transport::TCP { flags, .. } => flags,
            _                            => 0,
        }
    }
}


impl Default for Flow {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

impl Default for Window {
    fn default() -> Self {
        Window{
            size:  0,
            scale: 0,
        }
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.addr, self.port)
    }
}

pub fn tcp_window(p: &TcpPacket) -> Window {
    let mut scale = 1u8;

    if p.get_flags() & SYN == SYN {
        use pnet::packet::Packet;
        use pnet::packet::tcp::TcpOptionNumbers::WSCALE;

        for o in p.get_options_iter().filter(|o| o.get_number() == WSCALE) {
            if let &[n] = o.payload() {
                scale = n;
            }
        }
    }

    Window {
        size:  p.get_window() as u32,
        scale: scale,
    }
}

impl From<SocketAddr> for Addr {
    fn from(sa: SocketAddr) -> Self {
        Self {
            addr: sa.ip(),
            port: sa.port(),
        }
    }
}
