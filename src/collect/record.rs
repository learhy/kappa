use std::sync::Arc;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use crate::augment::Object;
use crate::capture::Flow;
use crate::process::Process;

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub flow: Flow,
    pub src:  Meta,
    pub dst:  Meta,
    pub srtt: Duration,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Meta {
    pub proc: Option<Arc<Process>>,
    pub node: Option<Arc<String>>,
    pub kube: Option<Arc<Object>>,
}
