use std::convert::{TryFrom, TryInto};
use anyhow::{Error, Result};
use libc::pid_t;
use procfs::process::{self, all_processes};
use tokio::task::spawn_blocking;
use super::{Process, CGroup};

impl Process {
    pub async fn scan() -> Result<Vec<Self>> {
        let procs = spawn_blocking(all_processes).await??.into_iter();
        Ok(procs.map(Self::try_from).collect::<Result<_>>()?)
    }

    pub fn load(pid: pid_t) -> Result<Self> {
        Ok(process::Process::new(pid)?.try_into()?)
    }
}

impl TryFrom<process::Process> for Process {
    type Error = Error;

    fn try_from(proc: process::Process) -> Result<Self, Self::Error> {
        let mut container = None;

        let cmdline = proc.cmdline()?;
        let cgroups = proc.cgroups()?;
        let comm    = proc.stat.comm;

        let cgroups = cgroups.into_iter().map(|c| {
            if c.pathname.starts_with("/kubepods/") {
                container = match c.pathname.split("/").last() {
                    Some(str) if str != "" => Some(str.to_owned()),
                    _                      => None,
                }
            }

            CGroup  {
                hierarchy:   c.hierarchy,
                controllers: c.controllers,
                path:        c.pathname,
            }
        }).collect();

        Ok(Process {
            pid:       proc.pid,
            comm:      comm,
            cmdline:   cmdline,
            cgroups:   cgroups,
            container: container,
        })
    }
}
