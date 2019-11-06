use std::fs::{self, File};
use std::io::{prelude::*, BufReader, ErrorKind};
use anyhow::Result;
use crate::process::{Process, CGroup};

pub fn lookup(pid: u32) -> Result<Process> {
    let comm    = or_default(comm(pid))?;
    let cmdline = or_default(cmdline(pid))?;
    let cgroups = or_default(cgroup(pid))?;

    let mut container = None;
    for cgroup in &cgroups {
        if cgroup.path.starts_with("/kubepods/") {
            container = match cgroup.path.split("/").last() {
                Some(str) if str != "" => Some(str.to_owned()),
                _                      => None,
            }
        }
    }

    Ok(Process {
        comm:      comm,
        cmdline:   cmdline,
        cgroups:   cgroups,
        pid:       pid,
        container: container,
    })
}

fn comm(pid: u32) -> Result<String> {
    let comm = fs::read(format!("/proc/{}/comm", pid))?;
    let comm = std::str::from_utf8(&comm)?;
    Ok(comm.trim().to_owned())
}

fn cmdline(pid: u32) -> Result<Vec<String>> {
    let cmd = fs::read(format!("/proc/{}/cmdline", pid))?;
    cmd.split(|&c| c == 0).map(|part| {
        Ok(String::from_utf8(part.to_vec())?)
    }).collect()
}

fn cgroup(pid: u32) -> Result<Vec<CGroup>> {
    let path = format!("/proc/{}/cgroup", pid);
    BufReader::new(File::open(path)?).lines().map(|line| {
        let line = line?;
        let mut split = line.split(':');
        let mut next  = || split.next().unwrap_or("");
        Ok(CGroup {
            hierarchy:   next().parse()?,
            controllers: next().split(',').map(str::to_owned).collect(),
            path:        next().to_owned(),
        })
    }).collect()
}

pub fn or_default<T: Default>(r: Result<T>) -> Result<T> {
    r.or_else(|e| {
        match e.downcast_ref::<std::io::Error>() {
            Some(e) if e.kind() == ErrorKind::NotFound => Ok(T::default()),
            _                                          => Err(e),
        }
    })
}
