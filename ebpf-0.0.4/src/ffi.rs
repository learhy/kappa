// Copyright (C) 2017 - Will Glozer. All rights reserved.

#![allow(non_camel_case_types)]

use std::os::raw::c_int;
use libc::off_t;

pub const BPF_PSEUDO_MAP_FD: u8 = 1;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bpf_insn {
    pub code: u8,
    pub regs: u8,
    pub off:  i16,
    pub imm:  i32,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum bpf_cmd {
    BPF_MAP_CREATE          = 0,
    BPF_MAP_LOOKUP_ELEM     = 1,
    BPF_MAP_UPDATE_ELEM     = 2,
    BPF_MAP_DELETE_ELEM     = 3,
    BPF_MAP_GET_NEXT_KEY    = 4,
    BPF_PROG_LOAD           = 5,
    BPF_OBJ_PIN             = 6,
    BPF_OBJ_GET             = 7,
    BPF_PROG_ATTACH         = 8,
    BPF_PROG_DETACH         = 9,
    BPF_PROG_TEST_RUN       = 10,
    BPF_PROG_GET_NEXT_ID    = 11,
    BPF_MAP_GET_NEXT_ID     = 12,
    BPF_PROG_GET_FD_BY_ID   = 13,
    BPF_MAP_GET_FD_BY_ID    = 14,
    BPF_OBJ_GET_INFO_BY_FD  = 15,
    BPF_PROG_QUERY          = 16,
    BPF_RAW_TRACEPOINT_OPEN = 17,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum bpf_map_type {
    BPF_MAP_TYPE_UNSPEC           = 0,
    BPF_MAP_TYPE_HASH             = 1,
    BPF_MAP_TYPE_ARRAY            = 2,
    BPF_MAP_TYPE_PROG_ARRAY       = 3,
    BPF_MAP_TYPE_PERF_EVENT_ARRAY = 4,
    BPF_MAP_TYPE_PERCPU_HASH      = 5,
    BPF_MAP_TYPE_PERCPU_ARRAY     = 6,
    BPF_MAP_TYPE_STACK_TRACE      = 7,
    BPF_MAP_TYPE_CGROUP_ARRAY     = 8,
    BPF_MAP_TYPE_LRU_HASH         = 9,
    BPF_MAP_TYPE_LRU_PERCPU_HASH  = 10,
    BPF_MAP_TYPE_LPM_TRIE         = 11,
    BPF_MAP_TYPE_ARRAY_OF_MAPS    = 12,
    BPF_MAP_TYPE_HASH_OF_MAPS     = 13,
    BPF_MAP_TYPE_DEVMAP           = 14,
    BPF_MAP_TYPE_SOCKMAP          = 15,
    BPF_MAP_TYPE_CPUMAP           = 16,
    BPF_MAP_TYPE_XSKMAP           = 17,
    BPF_MAP_TYPE_SOCKHASH         = 18,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum bpf_prog_type {
    BPF_PROG_TYPE_UNSPEC           = 0,
    BPF_PROG_TYPE_SOCKET_FILTER    = 1,
    BPF_PROG_TYPE_KPROBE           = 2,
    BPF_PROG_TYPE_SCHED_CLS        = 3,
    BPF_PROG_TYPE_SCHED_ACT        = 4,
    BPF_PROG_TYPE_TRACEPOINT       = 5,
    BPF_PROG_TYPE_XDP              = 6,
    BPF_PROG_TYPE_PERF_EVENT       = 8,
    BPF_PROG_TYPE_CGROUP_SKB       = 9,
    BPF_PROG_TYPE_CGROUP_SOCK      = 10,
    BPF_PROG_TYPE_LWT_IN           = 11,
    BPF_PROG_TYPE_LWT_OUT          = 12,
    BPF_PROG_TYPE_LWT_XMIT         = 13,
    BPF_PROG_TYPE_SOCK_OPS         = 14,
    BPF_PROG_TYPE_SK_SKB           = 15,
    BPF_PROG_TYPE_CGROUP_DEVICE    = 16,
    BPF_PROG_TYPE_SK_MSG           = 17,
    BPF_PROG_TYPE_RAW_TRACEPOINT   = 18,
    BPF_PROG_TYPE_CGROUP_SOCK_ADDR = 19,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union bpf_attr {
    pub map_create: bpf_map_create_arg,
    pub map_access: bpf_map_access_arg,
    pub prog_load:  bpf_prog_load_arg,
    pub get_id:     bpf_get_id_arg,
    pub obj_cmd:    bpf_obj_arg,
    pub info:       bpf_info_arg,
    _align:         [u64; 6usize],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct bpf_map_create_arg {
    pub map_type:     u32,
    pub key_size:     u32,
    pub val_size:     u32,
    pub max_entries:  u32,
    pub map_flags:    u32,
    pub inner_map_fd: u32,
    pub numa_node:    u32,
    pub map_name:     [u8; 16],
    pub map_ifindex:  u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct bpf_map_access_arg {
    pub map_fd: u32,
    pub key:    u64,
    pub op:     bpf_map_op,
    pub flags:  u64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union bpf_map_op {
    pub val:  u64,
    pub next: u64,
    _align:   u64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct bpf_prog_load_arg {
    pub prog_type:    u32,
    pub insn_cnt:     u32,
    pub insns:        u64,
    pub license:      u64,
    pub log_level:    u32,
    pub log_size:     u32,
    pub log_buf:      u64,
    pub kern_version: u32,
    pub prog_flags:   u32,
    pub prog_name:    [u8; 16],
    pub prog_ifindex: u32,
    pub attach_type:  u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct bpf_get_id_arg {
    pub op:         bpf_get_op,
    pub next_id:    u32,
    pub open_flags: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union bpf_get_op {
    pub start_id: u32,
    pub prog_id:  u32,
    pub map_id:   u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bpf_obj_arg {
    pub pathname: u64,
    pub bpf_fd:   u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bpf_info_arg {
    pub bpf_fd:   u32,
    pub info_len: u32,
    pub info:     u64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bpf_map_info {
    pub _type:       u32,
    pub id:          u32,
    pub key_size:    u32,
    pub val_size:    u32,
    pub max_entries: u32,
    pub map_flags:   u32,
    pub name:        [u8; 16],
    pub ifindex:     u32,
    pub netns_dev:   u64,
    pub netns_ino:   u64,
    _align:          u64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bpf_prog_info {
    pub _type:             u32,
    pub id:                u32,
    pub tag:               [u8; 8],
    pub jited_prog_len:    u32,
    pub xlated_prog_len:   u32,
    pub jited_prog_insns:  u64,
    pub xlated_prog_insns: u64,
    pub load_time:         u64,
    pub created_by_uid:    u32,
    pub nr_map_ids:        u32,
    pub map_ids:           u64,
    pub name:              [u8; 16],
    pub ifindex:           u32,
    pub netnds_dev:        u64,
    pub netns_ino:         u64,
    _align:                u64,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum bpf_func_id {
    BPF_FUNC_unspec               = 0,
    BPF_FUNC_map_lookup_elem      = 1,
    BPF_FUNC_map_update_elem      = 2,
    BPF_FUNC_map_delete_elem      = 3,
    BPF_FUNC_probe_read           = 4,
    BPF_FUNC_ktime_get_ns         = 5,
    BPF_FUNC_trace_printk         = 6,
    BPF_FUNC_get_prandom_u32      = 7,
    BPF_FUNC_get_smp_processor_id = 8,
    BPF_FUNC_skb_store_bytes      = 9,
    BPF_FUNC_l3_csum_replace      = 10,
    BPF_FUNC_l4_csum_replace      = 11,
    BPF_FUNC_tail_call            = 12,
    BPF_FUNC_clone_redirect       = 13,
    BPF_FUNC_get_current_pid_tgid = 14,
    BPF_FUNC_get_current_uid_gid  = 15,
    BPF_FUNC_get_current_comm     = 16,
    BPF_FUNC_get_cgroup_classid   = 17,
    BPF_FUNC_skb_vlan_push        = 18,
    BPF_FUNC_skb_vlan_pop         = 19,
    BPF_FUNC_skb_get_tunnel_key   = 20,
    BPF_FUNC_skb_set_tunnel_key   = 21,
    BPF_FUNC_perf_event_read      = 22,
    BPF_FUNC_redirect             = 23,
    BPF_FUNC_get_route_realm      = 24,
    BPF_FUNC_perf_event_output    = 25,
}

unsafe impl ::zero::Pod for bpf_insn           {}
unsafe impl ::zero::Pod for bpf_map_create_arg {}

impl From<u32> for bpf_map_type {
    fn from(n: u32) -> Self {
        unsafe { ::std::mem::transmute(n) }
    }
}

pub const AF_XDP:                         c_int = 44;
pub const SOL_XDP:                        c_int = 283;

pub const XDP_MMAP_OFFSETS:               c_int = 1;
pub const XDP_RX_RING:                    c_int = 2;
pub const XDP_TX_RING:                    c_int = 3;
pub const XDP_UMEM_REG:                   c_int = 4;
pub const XDP_UMEM_FILL_RING:             c_int = 5;
pub const XDP_UMEM_COMPLETION_RING:       c_int = 6;
pub const XDP_STATISTICS:                 c_int = 7;

pub const XDP_SHARED_UMEM:                u16   = 1 << 0;
pub const XDP_COPY:                       u16   = 1 << 1;
pub const XDP_ZEROCOPY:                   u16   = 1 << 2;

pub const XDP_PGOFF_RX_RING:              off_t = 0;
pub const XDP_PGOFF_TX_RING:              off_t = 0x80000000;
pub const XDP_UMEM_PGOFF_FILL_RING:       off_t = 0x100000000;
pub const XDP_UMEM_PGOFF_COMPLETION_RING: off_t = 0x180000000;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct sockaddr_xdp {
    pub sxdp_family:         u16,
    pub sxdp_flags:          u16,
    pub sxdp_ifindex:        u32,
    pub sxdp_queue_id:       u32,
    pub sxdp_shared_umem_fd: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct xdp_mmap_offsets {
    pub rx: xdp_ring_offset,
    pub tx: xdp_ring_offset,
    pub fr: xdp_ring_offset,
    pub cr: xdp_ring_offset,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct xdp_ring_offset {
    pub producer: u64,
    pub consumer: u64,
    pub desc:     u64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct xdp_umem_reg {
    pub addr:     u64,
    pub len:      u64,
    pub chunksz:  u32,
    pub headroom: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct xdp_statistics {
    pub rx_dropped:       u64,
    pub rx_invalid_descs: u64,
    pub tx_invalid_descs: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct xdp_desc {
    pub addr:    u64,
    pub len:     u32,
    pub options: u32,
}

impl Into<u64> for xdp_desc {
    fn into(self) -> u64 {
        self.addr
    }
}
