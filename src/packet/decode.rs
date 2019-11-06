use pnet::packet::{Packet as PacketExt, PacketSize};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::vlan::VlanPacket;
use pnet::packet::ethernet::{EthernetPacket, EtherType, EtherTypes};
use super::{Packet, Opaque};

pub fn decode<'a>(p: &'a EthernetPacket<'a>) -> (Option<u16>, Option<Packet<'a>>) {
    let mut ethertype = p.get_ethertype();
    let mut payload   = p.payload();
    let mut vlan      = None;

    while ethertype == EtherTypes::Vlan {
        if let Some(pkt) = VlanPacket::new(payload) {
            vlan      = Some(pkt.get_vlan_identifier());
            ethertype = pkt.get_ethertype();
            payload   = &payload[pkt.packet_size()..];
        } else {
            return (None, None)
        }
    }

    match ethertype {
        EtherTypes::Ipv4 => (vlan, ipv4(payload)),
        EtherTypes::Ipv6 => (vlan, ipv6(payload)),
        _                => (vlan, other(payload, ethertype)),
    }
}

fn ipv4(payload: &[u8]) -> Option<Packet> {
    let mut pkt = Ipv4Packet::new(payload)?;
    if pkt.get_next_level_protocol() == IpNextHeaderProtocols::Ipv4 {
        let n = pkt.get_header_length() as usize * 4;
        pkt = Ipv4Packet::new(&payload[n..])?;
    }
    Some(Packet::IPv4(pkt))
}

fn ipv6(payload: &[u8])-> Option<Packet> {
    Some(Packet::IPv6(Ipv6Packet::new(payload)?))
}

fn other(payload: &[u8], ethertype: EtherType)-> Option<Packet> {
    Opaque::new(ethertype.0, payload).map(Packet::Other)
}
