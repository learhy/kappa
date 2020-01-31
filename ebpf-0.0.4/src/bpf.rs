// Copyright (C) 2017 - Will Glozer. All rights reserved.

use std::error;
use std::fmt;
use std::mem;
use std::os::raw::{c_int, c_void};
use std::rc::Rc;
use errno::Errno;
use libc::ENOENT;
use sys::*;
use self::Error::*;

#[derive(Debug)]
pub struct Program {
    pub name: String,
    pub kind: Kind,
    pub fd:   c_int,
    pub maps: Vec<Rc<Map>>,
}

#[derive(Clone, Debug)]
pub enum Kind {
    Kprobe(String),
    Kretprobe(String),
    Socket,
    Tracepoint(String),
    XDP(String),
}

#[derive(Debug)]
pub struct Map {
    pub name:  String,
    pub fd:    c_int,
    pub ksize: usize,
    pub vsize: usize,
    pub limit: usize,
}

#[derive(Debug)]
pub enum Error {
    Size(usize, usize),
    Syscall(Errno),
}

impl Map {
    pub fn insert<K: Sized, V: Sized>(&self, k: &K, v: &V) -> Result<(), Error> {
        let key = self.check(k, self.ksize)?;
        let val = self.check(v, self.vsize)?;
        match bpf_update_elem(self.fd, key, val, 0) {
            Ok(_)  => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn lookup<K: Sized, V: Sized + Default>(&self, k: &K) -> Result<Option<V>, Error> {
        let key = self.check(k, self.ksize)?;
        let mut val = V::default();
        match bpf_lookup_elem(self.fd, key, &mut val as *mut _ as *mut c_void) {
            Ok(_)              => Ok(Some(val)),
            Err(Errno(ENOENT)) => Ok(None),
            Err(err)           => Err(err.into()),
        }
    }

    pub fn delete<K: Sized>(&self, k: &K) -> Result<(), Error> {
        let key = self.check(k, self.ksize)?;
        match bpf_delete_elem(self.fd, key) {
            Ok(_)  => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    fn check<T: Sized>(&self, v: &T, expect: usize) -> Result<*const c_void, Error> {
        match mem::size_of::<T>() {
            size if size == expect => Ok(v as *const _ as *const c_void),
            size                   => Err(Size(size, expect))
        }
    }
}

impl Drop for Program {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        close(self.fd);
    }
}

impl Drop for Map {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        close(self.fd);
    }
}

impl From<Errno> for Error {
    fn from(err: Errno) -> Self {
        Syscall(err)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Size(..)    => "invalid size",
            Syscall(..) => "syscall error",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match self {
            Size(..)    => None,
            Syscall(..) => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}
