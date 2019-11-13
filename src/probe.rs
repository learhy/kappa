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
use kentik_api::Client;
use crate::args::opt;
use crate::capture::{self, Sample, Sources};
use crate::export::Export;
use crate::link::{self, Links};
use crate::probes;
use crate::sockets::Procs;

pub fn probe(args: &ArgMatches) -> Result<()> {
    let email  = value_t!(args, "email",  String)?;
    let token  = value_t!(args, "token",  String)?;
    let device = value_t!(args, "device", String)?;
    let plan   = opt(args.value_of("plan"))?;

    let region = args.value_of("region");
    let _proxy = args.value_of("proxy");
    let kernel = args.value_of("kernel").and_then(Version::parse);

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

    let client = Client::new(&email, &token, region);

    let procs      = Procs::watch(kernel, shutdown.clone())?;
    let mut links  = Links::watch(shutdown.clone())?;
    let mut export = Export::new(client, &device, plan, procs.sockets())?;

    let (tx, rx) = bounded(1_000);
    let mut sources = Sources::new(config, tx);

    let timeout = Duration::from_millis(1);

    while !shutdown.load(Ordering::Acquire) {
        if let Ok(flows) = rx.recv_timeout(timeout) {
            export.export(flows)?;
        }

        while let Ok(Some(event)) = links.recv() {
            match event {
                link::Event::Add(link, mac) => sources.add(link, mac)?,
                link::Event::Del(link)      => sources.del(link),
            }
        }
    }

    if let Err(e) = probes::clear() {
        warn!("failed to clear probes {:?}", e);
    }

    Ok(())
}
