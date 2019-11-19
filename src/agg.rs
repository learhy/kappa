use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use clap::{ArgMatches, value_t};
use futures::prelude::*;
use log::{debug, error, warn};
use signal_hook::{iterator::Signals, SIGINT, SIGTERM, SIGUSR1};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio::time::Interval;
use tokio_serde::{self, formats::Json};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};
use kentik_api::Client;
use crate::args::opt;
use crate::collect::Record;
use crate::combine::Combine;
use crate::export::get_or_create_device;

pub fn agg(args: &ArgMatches) -> Result<()> {
    let email    = value_t!(args, "email",  String)?;
    let token    = value_t!(args, "token",  String)?;
    let device   = value_t!(args, "device", String)?;
    let plan     = opt(args.value_of("plan"))?;
    let region   = args.value_of("region");
    let interval = value_t!(args, "interval", u64)?;
    let addr     = value_t!(args, "addr", String)?;

    let interval = Duration::from_secs(interval);
    let client   = Arc::new(Client::new(&email, &token, region)?);
    let mut rt   = Runtime::new()?;

    let device = match rt.block_on(get_or_create_device(client.clone(), &device, plan)) {
        Ok(device) => device,
        Err(e)     => panic!("{:}", e),
    };

    let combine  = Arc::new(Combine::new(client, device));
    let combine2 = combine.clone();
    let combine3 = combine.clone();

    rt.spawn(async {
        match execute(addr, combine2).await {
            Ok(()) => debug!("agg finished"),
            Err(e) => error!("agg failed: {}", e),
        }
    });
    rt.spawn(export(interval, combine3));

    let signals = Signals::new(&[SIGINT, SIGTERM, SIGUSR1])?;
    for signal in signals.forever() {
        match signal {
            SIGINT | SIGTERM => break,
            SIGUSR1          => combine.dump(),
            _                => unreachable!(),
        }
    }

    drop(rt);

    Ok(())
}

async fn execute(addr: String, combine: Arc<Combine>) -> Result<()> {
    let mut listener = TcpListener::bind(&addr).await?;
    loop {
        let (sock, addr) = listener.accept().await?;
        debug!("connection from {}", addr);
        let combine = combine.clone();
        tokio::spawn(async move {
            match agent(sock, combine).await {
                Ok(()) => debug!("agent {} finished", addr),
                Err(e) => error!("agent {} error: {}", addr, e),
            }
        });
    }
}

async fn agent(sock: TcpStream, combine: Arc<Combine>) -> Result<()> {
    let mut length = LengthDelimitedCodec::new();
    length.set_max_frame_length(32 * 1024 * 1024);
    let framed = FramedRead::new(sock, length);
    let format = Json::<Vec<Record>>::default();

    let mut codec = tokio_serde::FramedRead::new(framed, format);

    while let Some(rs) = codec.try_next().await? {
        combine.combine(rs);
    }

    Ok(())
}

async fn export(interval: Duration, combine: Arc<Combine>) {
    let mut timer = Interval::new_interval(interval);
    while let Some(_) = timer.next().await {
        if let Err(e) = combine.export() {
            warn!("export failed: {}", e);
        }
    }
}
