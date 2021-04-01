use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use anyhow::Result;
use clap::{ArgMatches, value_t};
use log::{debug, error, warn};
use nixv::Version;
use regex::Regex;
use signal_hook::{iterator::Signals, consts::signal::{SIGINT, SIGTERM}};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{Receiver, channel};
use kentik_api::Client;
use crate::hostname;
use crate::agg::Request;
use crate::args::{opt, read};
use crate::capture::{self, Sample, Sources};
use crate::collect::Sink;
use crate::export::Export;
use crate::link::Links;
use crate::process::Procs;
use crate::sockets::Sockets;

pub fn probe(args: &ArgMatches) -> Result<()> {
    let node   = opt(args.value_of("node")).transpose().unwrap_or_else(hostname)?;
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

    let client = Client::new(&email, &token, region)?;

    let shutdown = Arc::new(AtomicBool::new(false));
    let (tx, rx) = channel(1_000);
    let rt     = Runtime::new()?;
    let handle = rt.handle().clone();


    let procs   = Procs::new();
    let sockets = Arc::new(Sockets::new(procs.clone(), kernel, code)?);

    procs.exec(&handle);
    let _guard = sockets.clone().watch(shutdown.clone())?;

    let links = Links::watch(&handle, shutdown.clone())?;
    let sink  = Sink::new(Arc::new(node), sockets, tx, handle);

    rt.spawn(async move {
        let sources = Sources::new(config, sink);
        match sources.exec(links).await {
            Ok(()) => debug!("source monitor finished"),
            Err(e) => error!("source monitor failed: {:?}", e),
        }
    });

    rt.spawn(async move {
        match export(client, device, plan, rx).await {
            Ok(()) => debug!("export finished"),
            Err(e) => debug!("export failed: {:?}", e),
        }
    });

    let mut signals = Signals::new(&[SIGINT, SIGTERM])?;
    for signal in signals.forever() {
        match signal {
            SIGINT | SIGTERM => break,
            _                => unreachable!(),
        }
    }

    shutdown.store(true, Ordering::SeqCst);
    rt.shutdown_timeout(Duration::from_secs(4));

    Ok(())
}

async fn export(
    client: Client,
    device: String,
    plan:   Option<u64>,
    mut rx: Receiver<Request>,
) -> Result<()> {
    let mut export = Export::new(client, &device, plan).await?;

    while let Some(request) = rx.recv().await {
        if let Request::Traffic(records) = request {
            debug!("exporting {} flows", records.len());
            match export.export(records).await {
                Ok(()) => (),
                Err(e) => warn!("export failed: {:?}", e),
            }
        }
    }
    Ok(())
}
