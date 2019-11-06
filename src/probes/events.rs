use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::os::raw::c_int;
use anyhow::Result;
use ebpf::bpf::{Kind, Program};
use perf::sys::*;
use perf::ffi::*;

const SYSFS: &str = "/sys/kernel/debug/tracing";

pub fn create(p: &Program) -> Result<Option<u64>> {
    fn kprobe(c: char, event: &str) -> Result<u64> {
        let name  = format!("{}_{}", c, event);
        let event = format!("{}:{} {}", c, name,  event);

        let path = format!("{}/kprobe_events", SYSFS);
        let mut file = create(&path)?;
        writeln!(file, "{}", event)?;

        Ok(read(&format!("{}/events/kprobes/{}/id", SYSFS, name))?)
    }

    fn tracepoint(event: &str) -> Result<u64> {
        Ok(read(&format!("{}/events/{}/id", SYSFS, event))?)
    }

    fn create(path: &str) -> Result<File> {
        Ok(OpenOptions::new().append(true).write(true).open(path)?)
    }

    fn read(path: &str) -> Result<u64> {
        let mut file = File::open(path)?;
        let mut line = String::new();
        let n = file.read_to_string(&mut line)?;
        Ok(line[..n-1].parse()?)
    }

    match p.kind {
        Kind::Kprobe(ref event)     => Ok(Some(kprobe('p', event)?)),
        Kind::Kretprobe(ref event)  => Ok(Some(kprobe('r', event)?)),
        Kind::Tracepoint(ref event) => Ok(Some(tracepoint(event)?)),
        _                           => Ok(None),
    }
}

pub fn attach(id: u64, pfd: c_int) -> Result<c_int> {
    let mut attr = perf_event_attr::default();
    attr.type_        = PERF_TYPE_TRACEPOINT;
    attr.config       = id;
    attr.sample_type  = PERF_SAMPLE_RAW;

    let fd = perf_event_open(&attr, -1, 0, -1, 0)?;

    perf_event_ioc_enable(fd)?;
    perf_event_ioc_set_bpf(fd, pfd)?;

    Ok(fd)
}

pub fn clear() -> Result<()> {
    File::create(format!("{}/kprobe_events", SYSFS))?;
    Ok(())
}
