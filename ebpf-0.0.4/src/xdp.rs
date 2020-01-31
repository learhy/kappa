// Copyright (C) 2018 - Will Glozer. All rights reserved.

use std::cmp::min;
use std::error;
use std::fmt;
use std::mem;
use std::ops;
use std::os::raw::c_int;
use std::ptr;
use std::sync::atomic::{compiler_fence, Ordering};
use errno::{errno, Errno};
use libc::{self, socklen_t, off_t, c_void, SOCK_RAW};
use ffi::*;

#[derive(Debug)]
pub struct Socket {
    pub fd:     c_int,
    pub rx:     Ring<xdp_desc>,
    pub tx:     Ring<xdp_desc>,
    pub fr:     Ring<u64>,
    pub cr:     Ring<u64>,
    pub frames: *mut u8,
}

#[derive(Debug)]
pub struct Ring<T> {
    pub mask:     u32,
    pub size:     u32,
    pub producer: Index,
    pub consumer: Index,
    pub ring:     *mut T,
    pub map:      *mut c_void,
}

#[derive(Copy, Clone, Debug)]
pub struct Index {
    index:  u32,
    shared: *mut u32
}

#[derive(Debug)]
pub struct Error(Errno);

impl Socket {
    pub fn new(descs: u32, frames: u32, framesz: u32, headroom: u32) -> Result<Self, Error> {
        let fd = socket()?;

        let descs  = descs.next_power_of_two();
        let frames = frames.next_power_of_two();

        let size = frames * framesz;
        let addr = alloc(size)?;

        let reg  = xdp_umem_reg {
            addr:     addr as u64,
            len:      size as u64,
            chunksz:  framesz,
            headroom: headroom,
        };

        setsockopt(fd, XDP_UMEM_REG, &reg)?;
        setsockopt(fd, XDP_UMEM_FILL_RING,       &(descs as c_int))?;
        setsockopt(fd, XDP_UMEM_COMPLETION_RING, &(descs as c_int))?;

        let off: xdp_mmap_offsets = getsockopt(fd, XDP_MMAP_OFFSETS)?;
        let fr = ring(descs, &off.fr, fd, XDP_UMEM_PGOFF_FILL_RING)?;
        let cr = ring(descs, &off.cr, fd, XDP_UMEM_PGOFF_COMPLETION_RING)?;

        setsockopt(fd, XDP_RX_RING, &(descs as c_int))?;
        setsockopt(fd, XDP_TX_RING, &(descs as c_int))?;

        let off: xdp_mmap_offsets = getsockopt(fd, XDP_MMAP_OFFSETS)?;
        let rx = ring(descs, &off.rx, fd, XDP_PGOFF_RX_RING)?;
        let tx = ring(descs, &off.tx, fd, XDP_PGOFF_TX_RING)?;

        Ok(Socket {
            fd:     fd,
            rx:     rx,
            tx:     tx,
            fr:     fr,
            cr:     cr,
            frames: addr as *mut u8,
        })
    }

    pub fn bind(&self, index: u32, queue: u32, flags: u16) -> Result<(), Error> {
        let sa = sockaddr_xdp {
            sxdp_family:         AF_XDP as u16,
            sxdp_flags:          flags,
            sxdp_ifindex:        index,
            sxdp_queue_id:       queue,
            sxdp_shared_umem_fd: 0,
        };
        Ok(bind(self.fd, &sa)?)
    }

    pub fn flush(&self) -> Result<usize, Error> {
        Ok(flush(self.fd)?)
    }

    pub fn statistics(&self) -> Result<xdp_statistics, Error> {
        Ok(getsockopt(self.fd, XDP_STATISTICS)?)
    }
}

impl<T: Copy> Ring<T> {
    pub fn consumable(&mut self, wanted: usize) -> usize {
        let mut entries = self.producer - self.consumer;

        if entries == 0 {
            self.producer.load();
            entries = self.producer - self.consumer;
        }

        min(entries as usize, wanted)
    }

    pub fn producible(&mut self, wanted: usize) -> usize {
        let mut entries = self.consumer - self.producer;

        if entries >= wanted as u32 {
            return wanted;
        }

        self.consumer.load();
        self.consumer += self.size;

        entries = self.consumer - self.producer;

        min(entries as usize, wanted)
    }

    pub fn consume(&mut self, ds: &mut [T]) -> usize {
        let n = self.consumable(ds.len());

        compiler_fence(Ordering::Acquire);

        if n > 0 {
            for d in &mut ds[0..n] {
                let index = *self.consumer;
                *d = self[index];
                self.consumer += 1;
            }

            compiler_fence(Ordering::Release);

            self.consumer.store();
        }

        n
    }

    pub fn produce<U: Copy + Into<T>>(&mut self, ds: &[U]) -> usize {
        let n = self.producible(ds.len());

        if n > 0 {
            for d in &ds[..n] {
                let index = *self.producer;
                self[index] = (*d).into();
                self.producer += 1;
            }

            compiler_fence(Ordering::Release);

            self.producer.store();
        }

        n
    }
}

impl Index {
    pub fn new(index: u32, shared: *mut u32) -> Self {
        Index {
            index:  index,
            shared: shared,
        }
    }

    #[inline(always)]
    fn load(&mut self) {
        unsafe {
            self.index = *self.shared;
        }
    }

    #[inline(always)]
    fn store(&mut self) {
        unsafe {
            *self.shared = self.index;
        }
    }
}

impl<T> ops::Index<u32> for Ring<T> {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: u32) -> &T {
        unsafe {
            &*self.ring.add((index & self.mask) as usize)
        }
    }

}

impl<T> ops::IndexMut<u32> for Ring<T> {
    #[inline(always)]
    fn index_mut(&mut self, index: u32) -> &mut T {
        unsafe {
            &mut *self.ring.add((index & self.mask) as usize)
        }
    }
}

impl ops::Deref for Index {
    type Target = u32;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.index
    }
}

impl ops::AddAssign<u32> for Index {
    #[inline(always)]
    fn add_assign(&mut self, that: u32) {
        self.index = self.index.wrapping_add(that);
    }
}

impl ops::Sub<Index> for Index {
    type Output = u32;

    #[inline(always)]
    fn sub(self, that: Index) -> u32 {
        self.index.wrapping_sub(that.index)
    }
}

fn socket() -> Result<c_int, Errno> {
    unsafe {
        match libc::socket(AF_XDP, SOCK_RAW, 0) {
            -1 => Err(errno()),
             n => Ok(n),
        }
    }
}

fn bind(fd: c_int, sa: &sockaddr_xdp) -> Result<(), Errno> {
    use libc::*;
    unsafe {
        let ptr = sa as *const _ as *const sockaddr;
        let len = mem::size_of_val(sa) as socklen_t;
        match bind(fd, ptr, len) {
            -1 => Err(errno()),
            _  => Ok(()),
        }
    }
}

fn flush(fd: c_int) -> Result<usize, Errno> {
    use libc::*;
    unsafe {
        match sendto(fd, ptr::null(), 0, MSG_DONTWAIT, ptr::null(), 0) as i32 {
            n if n >= 0               => Ok(n as usize),
            _ if errno().0 == EAGAIN  => Ok(0),
            _ if errno().0 == EBUSY   => Ok(0),
            _ if errno().0 == ENOBUFS => Ok(0),
            _                         => Err(errno()),
        }
    }
}

fn getsockopt<T>(fd: c_int, name: c_int) -> Result<T, Errno> {
    unsafe {
        let mut val: T = mem::zeroed();
        let mut len    = mem::size_of::<T>() as socklen_t;
        let mval = &mut val as *mut _ as *mut c_void;
        match libc::getsockopt(fd, SOL_XDP, name, mval, &mut len) {
            -1 => Err(errno()),
             _ => Ok(val),
        }
    }
}

fn setsockopt<T>(fd: c_int, name: c_int, val: &T) -> Result<(), Errno> {
    unsafe {
        let val = val as *const _ as *const c_void;
        let len = mem::size_of::<T>() as socklen_t;
        match libc::setsockopt(fd, SOL_XDP, name, val, len) {
            -1 => Err(errno()),
             _ => Ok(()),
        }
    }
}

fn alloc(size: u32) -> Result<*mut c_void, Errno> {
    use libc::*;
    unsafe {
        let pagesize = sysconf(_SC_PAGESIZE) as size_t;
        let mut addr = ptr::null_mut();
        let size     = size as size_t;
        match posix_memalign(&mut addr, pagesize, size) {
            0 => Ok(addr),
            n => Err(Errno(n)),
        }
    }
}

fn mmap<T>(count: u32, offset: u64, fd: c_int, kind: off_t) -> Result<*mut c_void, Errno> {
    use libc::*;
    unsafe {
        let offset = offset as usize;
        let count  = count  as usize;
        let size   = (offset + mem::size_of::<T>() * count) as size_t;
        let prot   = PROT_READ  | PROT_WRITE;
        let flags  = MAP_SHARED | MAP_POPULATE;

        match mmap(ptr::null_mut(), size, prot, flags, fd, kind) {
            ptr if ptr == MAP_FAILED => Err(errno()),
            ptr                      => Ok(ptr),
        }
    }
}

fn ring<T>(count: u32, off: &xdp_ring_offset, fd: c_int, kind: off_t) -> Result<Ring<T>, Errno> {
    unsafe {
        let map = mmap::<T>(count, off.desc, fd, kind)?;

        let cons = match kind {
            XDP_PGOFF_TX_RING        => count,
            XDP_UMEM_PGOFF_FILL_RING => count,
            _                        => 0,
        };

        let producer = map.add(off.producer as usize) as *mut u32;
        let consumer = map.add(off.consumer as usize) as *mut u32;
        let ring     = map.add(off.desc     as usize) as *mut T;

        let producer = Index::new(0,    producer);
        let consumer = Index::new(cons, consumer);

        Ok(Ring {
            mask:     count - 1,
            size:     count,
            producer: producer,
            consumer: consumer,
            ring:     ring,
            map:      map,
        })
    }
}

impl From<Errno> for Error {
    fn from(err: Errno) -> Self {
        Error(err)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "syscall error"
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}
