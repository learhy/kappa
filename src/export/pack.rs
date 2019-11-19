use std::net::IpAddr;
use anyhow::{Result, anyhow};
use capnp::{message::Builder, serialize_packed};
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

    for (index, Record { flow, src, dst }) in records.iter().enumerate() {
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

        let mut count = 0;
        let mut index = 0;

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

        let mut customs = msg.init_custom(count);

        if let Some(proc) = src {
            log::trace!("{} -> {}: {} ({})", flow.src, flow.dst, proc.comm, proc.pid);

            {
                let mut src_proc_pid = customs.reborrow().get(index);
                src_proc_pid.set_id(int01);
                src_proc_pid.init_value().set_uint32_val(proc.pid);
                index += 1;
            }

            {
                let mut src_proc_name = customs.reborrow().get(index);
                src_proc_name.set_id(str00);
                src_proc_name.init_value().set_str_val(&proc.comm);
                index += 1;
            }

            {
                let cmdline = proc.cmdline.join(" ");
                let mut src_proc_cmdline = customs.reborrow().get(index);
                src_proc_cmdline.set_id(str01);
                src_proc_cmdline.init_value().set_str_val(&cmdline);
                index += 1;
            }

            if let Some(id) = &proc.container {
                let mut src_proc_cont_id = customs.reborrow().get(index);
                src_proc_cont_id.set_id(str02);
                src_proc_cont_id.init_value().set_str_val(&id);
                index += 1;
            }
        }

        if let Some(proc) = dst {
            log::trace!("{} -> {}: {} ({})", flow.src, flow.dst, proc.comm, proc.pid);

            {
                let mut dst_proc_pid = customs.reborrow().get(index);
                dst_proc_pid.set_id(int02);
                dst_proc_pid.init_value().set_uint32_val(proc.pid);
                index += 1;
            }

            {
                let mut dst_proc_name = customs.reborrow().get(index);
                dst_proc_name.set_id(str03);
                dst_proc_name.init_value().set_str_val(&proc.comm);
                index += 1;
            }

            {
                let cmdline = proc.cmdline.join(" ");
                let mut dst_proc_cmdline = customs.reborrow().get(index);
                dst_proc_cmdline.set_id(str04);
                dst_proc_cmdline.init_value().set_str_val(&cmdline);
                index += 1;
            }

            if let Some(id) = &proc.container {
                let mut dst_proc_cont_id = customs.reborrow().get(index);
                dst_proc_cont_id.set_id(str05);
                dst_proc_cont_id.init_value().set_str_val(&id);
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
