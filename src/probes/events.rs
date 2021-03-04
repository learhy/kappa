use std::fs::{self, OpenOptions};
use std::io::prelude::*;
use std::os::raw::c_int;
use std::path::Path;
use anyhow::Result;
use log::warn;
use perf::sys::*;
use perf::ffi::*;

pub struct Event {
    name:    String,
    id:      u64,
    clear:   bool,
    perf_fd: Option<c_int>,
}

impl Event {
    pub fn kprobe(c: char, event: &str) -> Result<Self> {
        let name = format!("{}_{}", c, event);
        create_kprobe(c, &name, event)?;
        let id = event_id(&name, true)?;

        Ok(Self {
            name:    name,
            id:      id,
            clear:   true,
            perf_fd: None,
        })
    }

    pub fn tracepoint(event: &str) -> Result<Self> {
        Ok(Event {
            name:    event.to_owned(),
            id:      event_id(event, false)?,
            clear:   false,
            perf_fd: None,
        })
    }

    pub fn attach(mut self, prog_fd: c_int) -> Result<Self> {
        let mut attr = perf_event_attr::default();
        attr.type_       = PERF_TYPE_TRACEPOINT;
        attr.config      = self.id;
        attr.sample_type = PERF_SAMPLE_RAW;

        let perf_fd = perf_event_open(&attr, -1, 0, -1, 0)?;
        perf_event_ioc_enable(perf_fd)?;
        perf_event_ioc_set_bpf(perf_fd, prog_fd)?;

        self.perf_fd = Some(perf_fd);

        Ok(self)
    }

    pub fn detach(&self) {
        let Self { ref name, clear, perf_fd, .. } = *self;

        if let Some(fd) = perf_fd {
            if unsafe { libc::close(fd) } != 0 {
                warn!("error closing {} perf fd", name);
            }
        }

        if clear {
            if let Err(e) = clear_kprobe(name) {
                warn!("error clearing {}: {}", name, e);
            }
        }
    }
}

fn event_id(event: &str, kprobe: bool) -> Result<u64> {
    let mut path = Path::new(SYSFS).join("events");
    if kprobe { path.push("kprobes"); }
    path.push(event);
    path.push("id");
    Ok(fs::read_to_string(path)?.trim().parse()?)
}

fn create_kprobe(c: char, name: &str, event: &str) -> Result<()> {
    let path = Path::new(SYSFS).join("kprobe_events");
    let line = format!("{}:{} {}", c, name, event);
    let mut file = OpenOptions::new().append(true).open(path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

fn clear_kprobe(name: &str) -> Result<()> {
    let path = Path::new(SYSFS).join("kprobe_events");
    let line = format!("-:{}", name);
    let mut file = OpenOptions::new().append(true).open(path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

impl Drop for Event {
    fn drop(&mut self) {
        self.detach();
    }
}

const SYSFS: &str = "/sys/kernel/debug/tracing";
