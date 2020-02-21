use std::net::IpAddr;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Object {
    Pod(Pod),
    Service(Service),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pod {
    pub name:       String,
    pub ns:         String,
    pub labels:     String,
    pub ip:         IP,
    pub containers: Vec<Container>,
    pub workload:   Option<Workload>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service {
    pub name:   String,
    pub ns:     String,
    pub labels: String,
    pub ip:     IpAddr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "ip")]
pub enum IP {
    Host(IpAddr),
    Pod(IpAddr),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Container {
    pub name:  String,
    pub id:    String,
    pub image: String,
    pub ports: Vec<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workload {
    pub name: String,
    pub ns:   String,
}
