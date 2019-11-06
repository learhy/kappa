use pnet::packet::ethernet::EthernetPacket;
use pnet::packet::icmp::IcmpPacket;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::util::MacAddr;
use crate::packet::{self, Packet, Opaque};
use super::flow::*;
use crate::packet::Transport::*;

pub fn decode(mac: Option<MacAddr>, cap: pcap::Packet<'_>) -> Option<Flow> {
    let ts  = Timestamp(cap.header.ts);
    let eth = EthernetPacket::new(cap.data)?;

    let bytes = cap.header.len as usize - EthernetPacket::minimum_packet_size();
    let (vlan, pkt) = packet::decode(&eth);

    let pkt = pkt?;

    let eth = Ethernet {
        src:  eth.get_source(),
        dst:  eth.get_destination(),
        vlan: vlan,
    };

    let dir = match mac {
        Some(mac) if mac == eth.dst => Direction::In,
        Some(mac) if mac == eth.src => Direction::Out,
        _                           => Direction::Unknown,
    };

    pkt.transport(pkt.payload()).map(|transport| {
        let mut flow = match transport {
            TCP(ref p)   => tcp(eth, &pkt, p),
            UDP(ref p)   => udp(eth, &pkt, p),
            ICMP(ref p)  => icmp(eth, &pkt, p),
            Other(ref o) => ip(eth, &pkt, o),
        };

        flow.timestamp = ts;
        flow.bytes     = bytes;
        flow.direction = dir;

        flow
    })
}

fn tcp(eth: Ethernet, p: &Packet, tcp: &TcpPacket) -> Flow {
    let seq    = tcp.get_sequence();
    let flags  = tcp.get_flags();
    let window = tcp_window(tcp);

    Flow{
        protocol:  Protocol::TCP,
        ethernet:  eth,
        src:       Addr{addr: p.src(), port: tcp.get_source()},
        dst:       Addr{addr: p.dst(), port: tcp.get_destination()},
        tos:       p.tos(),
        transport: Transport::TCP{ seq, flags, window },
        .. Default::default()
    }
}

fn udp(eth: Ethernet, p: &Packet, udp: &UdpPacket) -> Flow {
    Flow{
        protocol:  Protocol::UDP,
        ethernet:  eth,
        src:       Addr{addr: p.src(), port: udp.get_source()},
        dst:       Addr{addr: p.dst(), port: udp.get_destination()},
        tos:       p.tos(),
        transport: Transport::UDP,
        .. Default::default()
    }
}

fn icmp(eth: Ethernet, p: &Packet, icmp: &IcmpPacket) -> Flow {
    let pack = ((icmp.get_icmp_type().0 as u16) << 8) | icmp.get_icmp_code().0 as u16;

    Flow{
        protocol:  Protocol::ICMP,
        ethernet:  eth,
        src:       Addr{addr: p.src(), port: 0   },
        dst:       Addr{addr: p.dst(), port: pack},
        tos:       p.tos(),
        transport: Transport::ICMP,
        .. Default::default()
    }
}

fn ip(eth: Ethernet, p: &Packet, o: &Opaque) -> Flow {
    Flow{
        protocol:  Protocol::Other(o.protocol),
        ethernet:  eth,
        src:       Addr{addr: p.src(), port: 0},
        dst:       Addr{addr: p.dst(), port: 0},
        tos:       p.tos(),
        transport: Transport::Other,
        .. Default::default()
    }
}
