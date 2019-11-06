use std::mem;
use std::os::raw::c_int;
use errno::errno;
use libc::{self, epoll_event, POLLIN, EPOLL_CTL_ADD};
use perf::{Error, map::Map};

pub struct Poll {
    fd:    c_int,
    ready: Vec<epoll_event>,
    maps:  Vec<Map>,
}

impl Poll {
    pub fn new(fds: &[c_int], pages: usize) -> Result<Self, Error> {
        let mut maps  = Vec::with_capacity(fds.len());
        let mut ready = Vec::with_capacity(fds.len());

        unsafe {
            ready.resize_with(fds.len(), || mem::zeroed());

            let epfd = create()?;

            for (index, &fd) in fds.iter().enumerate() {
                add(epfd, fd, &mut epoll_event {
                    events: POLLIN as u32,
                    u64:    index  as u64,
                })?;
                maps.push(Map::new(fd, pages)?);
            }

            Ok(Self { fd: epfd, ready, maps } )
        }
    }

    pub fn poll(&mut self, maps: &mut Vec<&mut Map>, timeout: i32) -> Result<usize, Error> {
        unsafe {
            let n = wait(self.fd, &mut self.ready, timeout)?;

            maps.clear();

            for event in &self.ready[..n] {
                let index = event.u64 as usize;
                let map   = self.maps.as_ptr().add(index);
                maps.push(mem::transmute(map));
            }

            Ok(n)
        }
    }
}

impl Drop for Poll {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

unsafe fn create() -> Result<c_int, Error> {
    match libc::epoll_create1(0) {
        -1 => Err(errno())?,
        fd => Ok(fd),
    }
}

unsafe fn add(epfd: c_int, fd: c_int, event: &mut epoll_event) -> Result<(), Error> {
    match libc::epoll_ctl(epfd, EPOLL_CTL_ADD, fd, event) {
        0 => Ok(()),
        _ => Err(errno())?,
    }
}

unsafe fn wait(epfd: c_int, events: &mut [epoll_event], timeout: i32) -> Result<usize, Error> {
    let count  = events.len() as c_int;
    let events = events.as_mut_ptr();
    match libc::epoll_wait(epfd, events, count, timeout as c_int) {
        -1 => Err(errno())?,
         n => Ok(n as usize),
    }
}
