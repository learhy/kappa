use std::sync::Arc;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use crate::capture::Flow;
use crate::sockets::Process;

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
    pub kube: Option<Arc<Kube>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Kube {
    pub pod:       Option<Name>,
    pub service:   Option<Name>,
    pub workload:  Option<Name>,
    pub container: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Name {
    pub name: String,
    pub ns:   String,
}
