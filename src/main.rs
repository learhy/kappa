use std::env;
use std::process;
use anyhow::{Error, Result};
use clap::{App, load_yaml};
use env_logger::Builder;
use jemallocator::Jemalloc;
use log::info;
use log::LevelFilter::*;
use kappa::{agent, agg, probe};

#[global_allocator]
static ALLOC: Jemalloc = Jemalloc;

fn main() -> Result<()> {
    let yaml = load_yaml!("args.yml");
    let ver  = env!("CARGO_PKG_VERSION");
    let args = App::from_yaml(&yaml).version(ver).get_matches();

    let (module, level) = match args.occurrences_of("verbose") {
        0 => (Some(module_path!()), Info),
        1 => (Some(module_path!()), Debug),
        2 => (Some(module_path!()), Trace),
        _ => (None,                 Trace),
    };
    Builder::from_default_env().filter(module, level).init();

    info!("initializing kappa {}", ver);

    match args.subcommand() {
        ("agent", Some(args)) => agent::agent(&args),
        ("agg",   Some(args)) => agg::agg(&args),
        ("probe", Some(args)) => probe::probe(&args),
        _                     => unreachable!(),
    }.unwrap_or_else(abort);

    Ok(())
}

fn abort(e: Error) {
    match e.downcast_ref::<clap::Error>() {
        Some(e) => println!("{}", e.message),
        None    => println!("{:?}", e),
    }
    process::exit(1);
}
