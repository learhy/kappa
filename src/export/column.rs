use std::sync::Arc;
use log::trace;
use crate::augment::{Object, Pod, Service};
use crate::collect::Meta;
use crate::sockets::Process;

pub struct Columns<'a> {
    pub proc: Option<Proc<'a>>,
    pub node: Option<&'a str>,
    pub kube: Option<Kube<'a>>
}

pub struct Proc<'a> {
    pub pid:       u32,
    pub comm:      &'a str,
    pub cmdline:   String,
    pub container: Option<&'a str>,
}

pub struct Kube<'a>  {
    pub name:      &'a str,
    pub ns:        &'a str,
    pub kind:      &'static str,
    pub labels:    &'a str,
    pub container: Option<Container<'a>>,
    pub workload:  Option<Workload<'a>>,
}

pub struct Container<'a> {
    pub name:  &'a str,
    pub id:    &'a str,
    pub image: &'a str,
}

pub struct Workload<'a> {
    pub name: &'a str,
    pub ns:   &'a str,
}

impl<'a> Columns<'a> {
    pub fn new(meta: &'a Meta) -> Self {
        let proc = meta.proc.as_ref().map(Arc::as_ref);
        let node = meta.node.as_ref();
        let kube = meta.kube.as_ref().map(Arc::as_ref);
        Self {
            proc: proc.map(|proc| Proc::new(proc)),
            node: node.map(|n| n.as_str()),
            kube: kube.map(|kube| Kube::new(kube, proc)),
        }
    }

    pub fn count(&self) -> u32 {
        let mut count = 0;
        self.proc.as_ref().map(|p| count += p.count());
        self.node.as_ref().map(|_| count += 1);
        self.kube.as_ref().map(|k| count += k.count());
        count
    }

    pub fn trace(&self, prefix: &str) {
        if let Some(Proc{ pid, comm, .. }) = &self.proc {
            trace!("{} proc {} ({})", prefix, comm, pid);
        }

        if let Some(Kube{ container: Some(container), .. }) = &self.kube {
            trace!("{} cont {} ({})", prefix, container.name, &container.id[..12])
        }

        if let Some(Kube{ name, kind, .. }) = &self.kube {
            trace!("{} kube {}/{}", prefix, kind, name);
        }
    }
}

impl<'a> Proc<'a> {
    fn new(proc: &'a Process) -> Self {
        let cmdline   = proc.cmdline.join(" ");
        let container = proc.container.as_ref().map(String::as_str);
        Self {
            pid:       proc.pid,
            comm:      &proc.comm,
            cmdline:   cmdline,
            container: container,
        }
    }

    fn count(&self) -> u32 {
        match self.container {
            None     => 3,
            Some(..) => 4,
        }
    }
}

impl<'a> Kube<'a> {
    fn new(kube: &'a Object, proc: Option<&'a Process> ) -> Self {
        match kube {
            Object::Pod(o)     => pod(o, proc),
            Object::Service(o) => service(o),
        }
    }

    fn count(&self) -> u32 {
        let mut count = 4;

        if let Some(..) = &self.container {
            count += 1;
        }

        if let Some(..) = &self.workload {
            count += 2;
        }

        count
    }
}

fn pod<'a>(pod: &'a Pod, proc: Option<&'a Process>) -> Kube<'a> {
    Kube {
        name:      &pod.name,
        ns:        &pod.ns,
        kind:      "pod",
        labels:    &pod.labels,
        container: container(pod, proc),
        workload:  None,
    }
}

fn service<'a>(svc: &'a Service) -> Kube<'a> {
    Kube {
        name:      &svc.name,
        ns:        &svc.ns,
        kind:      "service",
        labels:    &svc.labels,
        container: None,
        workload:  None,
    }
}

fn container<'a>(pod: &'a Pod, proc: Option<&'a Process>) -> Option<Container<'a>> {
    let id = proc?.container.as_ref()?.as_str();
    let c  = pod.containers.iter().find(|c| c.id == id)?;
    Some(Container {
        name:  c.name.as_str(),
        id:    c.id.as_str(),
        image: c.image.as_str(),
    })
}
