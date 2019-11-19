use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::capture::Flow;
use crate::sockets::Process;

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub flow: Flow,
    pub src:  Option<Arc<Process>>,
    pub dst:  Option<Arc<Process>>,
}