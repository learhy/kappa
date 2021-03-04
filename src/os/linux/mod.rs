use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Error;
use std::os::unix::io::{AsRawFd, RawFd};
use anyhow::{Result, anyhow};
use nell::{Family, Message, Netlink};
use nell::ffi::*;
use nell::sync::Socket;
use nell::sys::Bytes;
use Netlink::Msg;

pub fn findns(nsid: u32) -> Result<File> {
    let mut sock = Socket::new(Family::ROUTE)?;

    let lookup = |name: &OsStr| -> Option<Result<File>> {
        let name = name.to_str()?;
        let pid  = name.parse().ok()?;
        Some(getns(pid))
    };

    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let name  = entry.file_name();
        if let Some(result) = lookup(&name) {
            let file = result?;
            match netnsid(&mut sock, file.as_raw_fd())? {
                Some(id) if id == nsid => return Ok(file),
                _                      => continue,
            }
        }
    }

    Err(anyhow!("no process has nsid {}", nsid))
}

pub fn getns(pid: u32) -> Result<File> {
    Ok(File::open(&format!("/proc/{}/ns/net", pid))?)
}

pub fn setns(ns: &File) -> Result<()> {
    unsafe {
        match libc::setns(ns.as_raw_fd(), 0) {
            0 => Ok(()),
            _ => Err(Error::last_os_error())?,
        }
    }
}

const NETNSA_FD: u16 = 3;

#[derive(Default)]
#[repr(C)]
pub struct nsidmsg {
    pub msg:  rtgenmsg,
    pub _pad: u16,
    pub nla:  rtattr,
    pub val:   u32,
}

fn netnsid(sock: &mut Socket, fd: RawFd) -> Result<Option<u32>> {
    let mut msg = Message::<nsidmsg>::new(RTM_GETNSID);
    msg.set_flags(NLM_F_REQUEST);
    msg.nla.rta_len  = 8;
    msg.nla.rta_type = NETNSA_FD;
    msg.val          = fd as u32;

    sock.send(&msg)?;

    match sock.recv::<nsidmsg>()? {
        Msg(msg) => Ok(Some(msg.val)),
        _        => Ok(None),
    }
}

unsafe impl Bytes for nsidmsg {}
