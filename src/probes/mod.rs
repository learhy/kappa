use std::net::SocketAddr;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Socket {
    Accept(u32, SocketAddr, SocketAddr),
    Connect(u32, SocketAddr, SocketAddr),
    Close(u32, SocketAddr, SocketAddr),
}

mod events;
mod poll;
mod probes;
mod version;
mod trace;

pub use probes::Probes;
pub use events::{attach, clear, create};
pub use trace::trace;
