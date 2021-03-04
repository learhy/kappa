use std::fs::File;
use anyhow::{Result, anyhow};
use nell::Family;
use nell::sync::Socket;
use crate::os::getpid;
use super::{Link, links, findns, getns, setns};

#[derive(Debug, Default)]
pub struct Peer {
    pub index: u32,
    pub nsid:  u32,
}

pub fn peer(Peer { index, nsid }: Peer) -> Result<(File, Link)> {
    let netns = findns(nsid)?;
    let curns = getns(getpid())?;

    setns(&netns)?;
    let link = find(index);
    setns(&curns)?;

    Ok((netns, link?))
}

fn find(index: u32) -> Result<Link> {
    let mut sock  = Socket::new(Family::ROUTE)?;
    let links = links(&mut sock)?;

    let link = match links.into_iter().find(|l| l.index == index) {
        Some(link) => link,
        None       => return Err(anyhow!("no link at index {}", index)),
    };

    Ok(link)
}
