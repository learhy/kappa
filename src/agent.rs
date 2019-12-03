use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use anyhow::Result;
use clap::{ArgMatches, value_t};
use crossbeam_channel::bounded;
use log::warn;
use nixv::Version;
use regex::Regex;
use signal_hook::{flag::register, SIGINT, SIGTERM, SIGUSR1};
use tokio::runtime::Runtime;
use crate::args::{opt, read};
use crate::capture::{self, Sample, Sources};
use crate::collect::Collect;
use crate::link::{Event, Links};
use crate::probes;
use crate::sockets::Procs;

pub fn agent(args: &ArgMatches) -> Result<()> {
    let agg      = value_t!(args, "agg", String)?;
    let kernel   = args.value_of("kernel").and_then(Version::parse);
    let interval = value_t!(args, "interval", u64)?;
    let sample   = opt(args.value_of("sample"))?.unwrap_or(Sample::None);

    let code = opt(args.value_of("bytecode"))?.map(read).transpose()?;

    let capture  = value_t!(args, "capture", String)?;
    let exclude  = args.value_of("exclude").unwrap_or("^$");

    let config  = capture::Config {
        capture:     Regex::new(&capture)?,
        exclude:     Regex::new(&exclude)?,
        interval:    Duration::from_secs(interval),
        buffer_size: 10_000_000,
        sample:      sample,
        snaplen:     128,
        promisc:     true,
    };

    let shutdown = Arc::new(AtomicBool::new(false));
    let dump     = Arc::new(AtomicBool::new(false));
    register(SIGTERM, shutdown.clone())?;
    register(SIGINT,  shutdown.clone())?;
    register(SIGUSR1, dump.clone())?;

    let rt          = Runtime::new()?;
    let procs       = Procs::watch(kernel, code, shutdown.clone())?;
    let mut links   = Links::watch(shutdown.clone())?;
    let mut collect = Collect::new(agg, procs.sockets(), &rt, dump);

    let (tx, rx) = bounded(1_000);
    let mut sources = Sources::new(config, tx);

    let timeout = Duration::from_millis(1);

    while !shutdown.load(Ordering::Acquire) {
        if let Ok(flows) = rx.recv_timeout(timeout) {
            collect.collect(flows)?;
        }

        while let Ok(Some(event)) = links.recv() {
            match event {
                Event::Add(add)       => sources.add(add)?,
                Event::Delete(link)   => sources.del(link),
                Event::Error(link, e) => warn!("link {} error: {}", link, e),
            }
        }
    }

    drop(rt);

    if let Err(e) = probes::clear() {
        warn!("failed to clear probes {:?}", e);
    }

    Ok(())
}
