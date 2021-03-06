use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use anyhow::Result;
use clap::{ArgMatches, value_t};
use crossbeam_channel::bounded;
use log::warn;
use nixv::Version;
use regex::Regex;
use signal_hook::{flag::register, consts::signal::{SIGINT, SIGTERM}};
use kentik_api::Client;
use crate::args::{opt, read};
use crate::capture::{self, Sample, Sources};
use crate::export::Export;
use crate::link::{Event, Links};
use crate::sockets::Procs;

pub fn probe(args: &ArgMatches) -> Result<()> {
    let node   = opt(args.value_of("node"))?.map(Arc::new);
    let email  = value_t!(args, "email",  String)?;
    let token  = value_t!(args, "token",  String)?;
    let device = value_t!(args, "device", String)?;
    let plan   = opt(args.value_of("plan"))?;

    let region = args.value_of("region");
    let _proxy = args.value_of("proxy");
    let kernel = args.value_of("kernel").and_then(Version::parse);

    let code = opt(args.value_of("bytecode"))?.map(read).transpose()?;

    let interval = value_t!(args, "interval", u64)?;
    let sample   = opt(args.value_of("sample"))?.unwrap_or(Sample::None);

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
    register(SIGTERM, shutdown.clone())?;
    register(SIGINT,  shutdown.clone())?;

    let client = Client::new(&email, &token, region)?;

    let procs      = Procs::watch(kernel, code, shutdown.clone())?;
    let mut links  = Links::watch(shutdown.clone())?;
    let mut export = Export::new(client, &device, plan, procs.sockets())?;

    let (tx, rx) = bounded(1_000);
    let mut sources = Sources::new(config, tx);

    let timeout = Duration::from_millis(5);

    while !shutdown.load(Ordering::Acquire) {
        if let Ok(flows) = rx.recv_timeout(timeout) {
            export.export(flows, node.clone())?;
        }

        while let Ok(Some(event)) = links.recv() {
            match event {
                Event::Add(add)       => sources.add(add)?,
                Event::Delete(link)   => sources.del(link),
                Event::Error(link, e) => warn!("link {} error: {}", link, e),
            }
        }
    }

    Ok(())
}
