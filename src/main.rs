use std::convert::TryInto;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use anyhow::Result;
use clap::{App, load_yaml, value_t, values_t};
use crossbeam_channel::bounded;
use env_logger::Builder;
use log::info;
use log::LevelFilter::*;
use nixv::Version;
use pcap::Capture;
use signal_hook::{flag::register, SIGINT, SIGTERM};
use kentik_api::Client;
use kappa::capture::{self, Source};
use kappa::export::Export;
use kappa::process::Monitor;

fn main() -> Result<()> {
    let yaml = load_yaml!("args.yml");
    let ver  = env!("CARGO_PKG_VERSION");
    let args = App::from_yaml(&yaml).version(ver).get_matches();

    let email  = value_t!(args, "email",  String)?;
    let token  = value_t!(args, "token",  String)?;
    let device = value_t!(args, "device", String)?;
    let plan   = opt(args.value_of("plan"))?;

    let region = args.value_of("region");
    let _proxy = args.value_of("proxy");
    let kernel = args.value_of("kernel").and_then(Version::parse);

    let interval = value_t!(args, "interval", u64)?;
    let captures = values_t!(args, "interface", String)?.into_iter().map(|name| {
        let (mac, dev) = capture::lookup(&name)?;
        let cap = Capture::from_device(dev)?
            .buffer_size(10_000_000)
            .timeout((interval * 1000).try_into()?)
            .snaplen(128)
            .promisc(true);
        Ok(Source::new(cap, mac, name))
    }).collect::<Result<Vec<_>>>()?;

    let (module, level) = match args.occurrences_of("verbose") {
        0 => (Some(module_path!()), Info),
        1 => (Some(module_path!()), Debug),
        2 => (Some(module_path!()), Trace),
        _ => (None,                 Trace),
    };
    Builder::from_default_env().filter(module, level).init();

    info!("initializing kappa {}", ver);

    let shutdown = Arc::new(AtomicBool::new(false));
    register(SIGTERM, shutdown.clone())?;
    register(SIGINT,  shutdown.clone())?;

    let client = Client::new(&email, &token, region);

    let mut export  = Export::new(client, &device, plan)?;
    let mut monitor = Monitor::start(kernel, shutdown.clone())?;

    let (tx, rx) = bounded(1_000);
    let captures = captures.into_iter().map(|source| {
        let interval = Duration::from_secs(interval);
        source.start(interval, tx.clone())
    }).collect::<Result<Vec<_>>>()?;

    let timeout = Duration::from_millis(1);

    while !shutdown.load(Ordering::Acquire) {
        if let Ok(flows) = rx.recv_timeout(timeout) {
            export.export(flows)?;
        }

        while let Ok(Some(event)) = monitor.recv() {
            export.record(event);
        }
    }

    Ok(())
}

fn opt<T: FromStr>(arg: Option<&str>) -> Result<Option<T>> {
    Ok(arg.map(|s| T::from_str(s).map_err(|_| {
        let msg  = format!("invalid argument value '{}'", s);
        let kind = clap::ErrorKind::InvalidValue;
        clap::Error::with_description(&msg, kind)
    })).transpose()?)
}
