use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use anyhow::Result;
use clap::{ArgMatches, value_t, values_t};
use crossbeam_channel::bounded;
use log::warn;
use nixv::Version;
use regex::Regex;
use signal_hook::{flag::register, SIGINT, SIGTERM};
use tokio::runtime::Runtime;
use crate::args::opt;
use crate::capture::{self, Sample, Sources};
use crate::collect::Collect;
use crate::link::{self, Links};
use crate::probes;
use crate::sockets::Procs;

pub fn agent(args: &ArgMatches) -> Result<()> {
    let agg      = value_t!(args, "agg", String)?;
    let kernel   = args.value_of("kernel").and_then(Version::parse);
    let interval = value_t!(args, "interval", u64)?;
    let sample   = opt(args.value_of("sample"))?.unwrap_or(Sample::None);

    let capture  = values_t!(args, "capture", String)?.join("|");
    let exclude  = args.values_of("exclude").map(|vs| {
        vs.map(String::from).collect::<Vec<_>>().join("|")
    }).unwrap_or_else(|| "^$".to_string());

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
    register(SIGTERM, shutdown.clone())?;
    register(SIGINT,  shutdown.clone())?;

    let rt          = Runtime::new()?;
    let procs       = Procs::watch(kernel, shutdown.clone())?;
    let mut links   = Links::watch(shutdown.clone())?;
    let mut collect = Collect::new(agg, procs.sockets(), &rt);

    let (tx, rx) = bounded(1_000);
    let mut sources = Sources::new(config, tx);

    let timeout = Duration::from_millis(1);

    while !shutdown.load(Ordering::Acquire) {
        if let Ok(flows) = rx.recv_timeout(timeout) {
            collect.collect(flows)?;
        }

        while let Ok(Some(event)) = links.recv() {
            match event {
                link::Event::Add(link, mac) => sources.add(link, mac)?,
                link::Event::Del(link)      => sources.del(link),
            }
        }
    }

    drop(rt);

    if let Err(e) = probes::clear() {
        warn!("failed to clear probes {:?}", e);
    }

    Ok(())
}
