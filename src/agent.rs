use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use anyhow::Result;
use clap::{ArgMatches, value_t};
use log::{debug, error};
use nixv::Version;
use regex::Regex;
use signal_hook::{iterator::Signals, consts::signal::{SIGINT, SIGTERM, SIGUSR1}};
use tokio::runtime::Runtime;
use crate::args::{opt, read};
use crate::capture::{Config, Sample, Sources};
use crate::collect::Collect;
use crate::link::Links;
use crate::sockets::Procs;

pub fn agent(args: &ArgMatches) -> Result<()> {
    let agg      = value_t!(args, "agg", String)?;
    let node     = opt(args.value_of("node"))?;
    let kernel   = args.value_of("kernel").and_then(Version::parse);
    let interval = value_t!(args, "interval", u64)?;
    let sample   = opt(args.value_of("sample"))?.unwrap_or(Sample::None);

    let code = opt(args.value_of("bytecode"))?.map(read).transpose()?;

    let capture  = value_t!(args, "capture", String)?;
    let exclude  = args.value_of("exclude").unwrap_or("^$");

    let config = Config {
        capture:     Regex::new(&capture)?,
        exclude:     Regex::new(&exclude)?,
        interval:    Duration::from_secs(interval),
        buffer_size: 10_000_000,
        sample:      sample,
        snaplen:     128,
        promisc:     true,
    };

    let shutdown = Arc::new(AtomicBool::new(false));
    let rt       = Runtime::new()?;
    let handle   = rt.handle().clone();

    let mut procs = Procs::new(kernel, code)?;
    procs.watch(shutdown.clone())?;

    let links   = Links::watch(&handle, shutdown.clone())?;
    let collect = Collect::new(agg, procs.sockets(), handle, node);
    let dump    = collect.dump();

    rt.spawn(async move {
        let sources = Sources::new(config, collect.sink());
        match sources.exec(links).await {
            Ok(()) => debug!("source monitor finished"),
            Err(e) => error!("source monitor failed: {}", e),
        }
    });

    let toggle = || {
        let state = dump.load(Ordering::SeqCst);
        dump.store(!state, Ordering::SeqCst);
    };

    let mut signals = Signals::new(&[SIGINT, SIGTERM, SIGUSR1])?;
    for signal in signals.forever() {
        match signal {
            SIGINT | SIGTERM => break,
            SIGUSR1          => toggle(),
            _                => unreachable!(),
        }
    }

    shutdown.store(true, Ordering::SeqCst);

    drop(rt);

    Ok(())
}
