pub use pnet::util::MacAddr;

#[derive(Debug)]
pub enum Event {
    Add(String, Option<MacAddr>),
    Del(String),
}

pub use monitor::Links;

mod monitor;
