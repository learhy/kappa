use std::net::IpAddr;
use anyhow::{Result, anyhow};
use capnp::{message::Builder, serialize_packed, struct_list};
use pnet::{packet::PrimitiveValues, util::MacAddr};
use kentik_api::Device;
use crate::chf_capnp::*;
use crate::capture::{Direction, Protocol};
use crate::collect::Record;

pub fn pack(device: &Device, records: Vec<Record>) -> Result<Vec<u8>> {
    let column = |name: &str| {
        match device.customs.iter().find(|c| c.name == name) {
            Some(c) => Ok(c.id as u32),
            None    => Err(anyhow!("missing custom column '{}'", name)),
        }
    };

    let lat   = column("APPL_LATENCY_MS")?;
    let app   = column("APP_PROTOCOL")?;
    let _nt00 = column("INT00")?;
    let int01 = column("INT01")?;
    let int02 = column("INT02")?;
    let str00 = column("STR00")?;
    let str01 = column("STR01")?;
    let str02 = column("STR02")?;
    let str03 = column("STR03")?;
    let str04 = column("STR04")?;
    let str05 = column("STR05")?;

    let mut msg  = Builder::new_default();
    let root = msg.init_root::<packed_c_h_f::Builder>();
    let mut msgs = root.init_msgs(records.len() as u32);

    for (index, Record { flow, src, dst, srtt }) in records.iter().enumerate() {
        let mut msg = msgs.reborrow().get(index as u32);

        let src_eth_mac = pack_mac(&flow.ethernet.src);
        let dst_eth_mac = pack_mac(&flow.ethernet.dst);

        msg.set_src_eth_mac(src_eth_mac);
        msg.set_dst_eth_mac(dst_eth_mac);

        msg.set_protocol(match flow.protocol {
            Protocol::ICMP     => 1,
            Protocol::TCP      => 6,
            Protocol::UDP      => 17,
            Protocol::Other(n) => n as u32,
        });

        match flow.src.addr {
            IpAddr::V4(ip) => msg.set_ipv4_src_addr(ip.into()),
            IpAddr::V6(ip) => msg.set_ipv6_src_addr(&ip.octets()),
        };

        match flow.dst.addr {
            IpAddr::V4(ip) => msg.set_ipv4_dst_addr(ip.into()),
            IpAddr::V6(ip) => msg.set_ipv6_dst_addr(&ip.octets()),
        };

        msg.set_l4_src_port(flow.src.port as u32);
        msg.set_l4_dst_port(flow.dst.port as u32);
        msg.set_tos(flow.tos as u32);
        msg.set_tcp_flags(flow.tcp_flags() as u32);
        msg.set_sample_rate(flow.sample);

        match flow.direction {
            Direction::In => {
                msg.set_in_pkts(flow.packets as u64);
                msg.set_in_bytes(flow.bytes as u64);
                msg.set_input_port(dst_eth_mac as u32 & 0xFFFF);
                msg.set_vlan_in(flow.ethernet.vlan.unwrap_or(0) as u32);
            },
            Direction::Out | Direction::Unknown => {
                msg.set_out_pkts(flow.packets as u64);
                msg.set_out_bytes(flow.bytes as u64);
                msg.set_output_port(dst_eth_mac as u32 & 0xFFFF);
                msg.set_vlan_out(flow.ethernet.vlan.unwrap_or(0) as u32);
            }
        };

        let mut count = 2;

        if let Some(proc) = src {
            count += match proc.container {
                Some(_) => 4,
                None    => 3,
            }
        }

        if let Some(proc) = dst {
            count += match proc.container {
                Some(_) => 4,
                None    => 3,
            }
        }

        let mut customs = Customs::new(msg.init_custom(count));

        let srtt = srtt.as_millis() as u32;
        customs.next(app, |v| v.set_uint32_val(1));
        customs.next(lat, |v| v.set_uint32_val(srtt));


        if let Some(proc) = src {
            log::trace!("{} -> {}: {} ({})", flow.src, flow.dst, proc.comm, proc.pid);

            customs.next(int01, |v| v.set_uint32_val(proc.pid));
            customs.next(str00, |v| v.set_str_val(&proc.comm));
            customs.next(str01, |v| v.set_str_val(&proc.cmdline.join(" ")));
            if let Some(id) = &proc.container {
                customs.next(str02, |v| v.set_str_val(&id));
            }
        }

        if let Some(proc) = dst {
            log::trace!("{} -> {}: {} ({})", flow.src, flow.dst, proc.comm, proc.pid);

            customs.next(int02, |v| v.set_uint32_val(proc.pid));
            customs.next(str03, |v| v.set_str_val(&proc.comm));
            customs.next(str04, |v| v.set_str_val(&proc.cmdline.join(" ")));
            if let Some(id) = &proc.container {
                customs.next(str05, |v| v.set_str_val(&id));
            }
        }
    }

    let mut vec = Vec::new();
    vec.resize_with(80, Default::default);
    serialize_packed::write_message(&mut vec, &msg)?;

    Ok(vec)
}

fn pack_mac(mac: &MacAddr) -> u64 {
    let prims = mac.to_primitive_values();
    (prims.0 as u64) << 40 |
    (prims.1 as u64) << 32 |
    (prims.2 as u64) << 24 |
    (prims.3 as u64) << 16 |
    (prims.4 as u64) << 8  |
    (prims.5 as u64)
}

struct Customs<'a> {
    builder: struct_list::Builder<'a, custom::Owned>,
    index:   u32,
}

impl<'a> Customs<'a> {
    fn new(b: struct_list::Builder<'a, custom::Owned>) -> Self {
        Self {
            builder: b,
            index:   0,
        }
    }

    fn next<F: Fn(&mut custom::value::Builder)>(&mut self, id: u32, f: F) {
        let mut custom = self.builder.reborrow().get(self.index);
        custom.set_id(id);
        f(&mut custom.init_value());
        self.index += 1;
    }
}
