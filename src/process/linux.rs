use std::convert::{TryFrom, TryInto};
use std::time::SystemTime;
use anyhow::{Error, Result};
use libc::pid_t;
use procfs::{Meminfo, boot_time_secs, ticks_per_second, page_size};
use procfs::process::{self, all_processes, Stat};
use tokio::task::spawn_blocking;
use super::{Process, CGroup, Stats};

impl Process {
    pub async fn scan() -> Result<Vec<Self>> {
        let procs = spawn_blocking(all_processes).await??.into_iter();
        Ok(procs.map(Self::try_from).collect::<Result<_>>()?)
    }

    pub fn load(pid: pid_t) -> Result<Self> {
        Ok(process::Process::new(pid)?.try_into()?)
    }

    pub fn stats(&self) -> Result<Stats> {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
        let tps = u64::try_from(ticks_per_second()?)?;

        let boot_time = boot_time_secs()?;
        let pagesize  = u64::try_from(page_size()?)?;
        let meminfo   = Meminfo::new()?;
        let uptime    = now - boot_time;

        let proc = process::Process::new(self.pid)?;
        let stat = proc.stat;

        Ok(Stats {
            parent: stat.ppid,
            uid:    proc.owner,
            pcpu:   pcpu(&stat, uptime, tps),
            pmem:   pmem(&stat, &meminfo, pagesize),
        })
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

fn pcpu(stat: &Stat, uptime: u64, tps: u64) -> f64 {
    let systime = (stat.stime as f64) / tps as f64;
    let usrtime = (stat.utime as f64) / tps as f64;
    let cputime = systime + usrtime;

    let starttime = (stat.starttime as f64) / tps as f64;
    let runtime   = uptime as f64 - starttime;

    100.0 * (cputime / runtime)
}

fn pmem(stat: &Stat, meminfo: &Meminfo, pagesize: u64) -> f64 {
    let rss   = (stat.rss as u64 * pagesize) as f64;
    let total = meminfo.mem_total as f64;
    100.0 * (rss / total)
}
