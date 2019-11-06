use std::fs::File;
use std::io::Read;
use std::str;
use std::thread;
use log::trace;

pub fn trace() {
    thread::spawn(|| {
        let mut file = File::open("/sys/kernel/debug/tracing/trace_pipe").unwrap();
        let mut buf  = [0u8; 4096];
        while let Ok(n) = file.read(&mut buf) {
            trace!("{}", str::from_utf8(&buf[..n]).unwrap());
        }
    });
}
