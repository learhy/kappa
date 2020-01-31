// Copyright (C) 2017 - Will Glozer. All rights reserved.

use std::error;
use std::ffi::CStr;
use std::fmt;
use std::mem;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use errno::{errno, Errno};
use libc::{self, syscall, SYS_bpf};
use ffi::*;
use ffi::bpf_cmd::*;
use self::Error::*;

#[derive(Debug)]
pub enum Error {
    Unsafe(String),
    Other(Errno),
}

pub fn bpf_create_map(arg: &bpf_map_create_arg) -> Result<c_int, Errno> {
    let mut attr = bpf_attr();
    attr.map_create = *arg;
    bpf(BPF_MAP_CREATE, &attr)
}

pub fn bpf_update_elem(fd: c_int, key: *const c_void, val: *const c_void, flags: u64) -> Result<(), Errno> {
    let mut attr = bpf_attr();
    attr.map_access = bpf_map_access_arg {
        map_fd: fd  as u32,
        key:    key as u64,
        op:     bpf_map_op { val: val as u64 },
        flags:  flags,
    };
    bpf(BPF_MAP_UPDATE_ELEM, &attr)?;
    Ok(())
}

pub fn bpf_lookup_elem(fd: c_int, key: *const c_void, val: *mut c_void) -> Result<(), Errno> {
    let mut attr = bpf_attr();
    attr.map_access = bpf_map_access_arg {
        map_fd: fd  as u32,
        key:    key as u64,
        op:     bpf_map_op { val: val as u64 },
        flags:  0,
    };
    bpf(BPF_MAP_LOOKUP_ELEM, &attr)?;
    Ok(())
}

pub fn bpf_delete_elem(fd: c_int, key: *const c_void) -> Result<(), Errno> {
    let mut attr = bpf_attr();
    attr.map_access = bpf_map_access_arg {
        map_fd: fd  as u32,
        key:    key as u64,
        op:     bpf_map_op { val: 0 },
        flags:  0,
    };
    bpf(BPF_MAP_DELETE_ELEM, &attr)?;
    Ok(())
}

pub fn bpf_get_next_key(fd: c_int, key: *const c_void, next: *mut c_void) -> Result<c_int, Errno> {
    let mut attr = bpf_attr();
    attr.map_access = bpf_map_access_arg {
        map_fd: fd   as u32,
        key:    key  as u64,
        op:     bpf_map_op { next: next as u64 },
        flags:  0,
    };
    bpf(BPF_MAP_GET_NEXT_KEY, &attr)
}

pub fn bpf_prog_load(arg: &bpf_prog_load_arg, log: &mut [u8]) -> Result<c_int, Error> {
    let mut attr = bpf_attr();

    attr.prog_load = bpf_prog_load_arg {
        log_buf:   log.as_mut_ptr() as u64,
        log_size:  (log.len() - 1)  as u32,
        log_level: 1,
        .. *arg
    };

    bpf(BPF_PROG_LOAD, &attr).map_err(|err| {
        let cause = || -> String {
            let ptr  = log.as_ptr() as *const c_char;
            let cstr = unsafe { CStr::from_ptr(ptr) };
            cstr.to_string_lossy().into_owned()
        };

        match err {
            Errno(libc::EACCES) => Error::Unsafe(cause()),
            errno               => Error::Other(errno),
        }
    })
}

pub fn bpf_obj_get(path: *const c_char) -> Result<c_int, Errno> {
    let mut attr = bpf_attr();
    attr.obj_cmd = bpf_obj_arg {
        bpf_fd:   0,
        pathname: path as u64,
    };
    bpf(BPF_OBJ_GET, &attr)
}

pub fn bpf_obj_pin(fd: c_int, path: *const c_char) -> Result<(), Errno> {
    let mut attr = bpf_attr();
    attr.obj_cmd = bpf_obj_arg {
        bpf_fd:   fd   as u32,
        pathname: path as u64,
    };
    bpf(BPF_OBJ_PIN, &attr)?;
    Ok(())
}

pub fn bpf_map_get_next_id(start_id: u32) -> Result<u32, Errno> {
    let mut attr = bpf_attr();
    attr.get_id = bpf_get_id_arg {
        op:         bpf_get_op { start_id: start_id },
        next_id:    0,
        open_flags: 0,
    };
    bpf(BPF_MAP_GET_NEXT_ID, &attr)?;
    Ok(unsafe { attr.get_id.next_id })
}

pub fn bpf_prog_get_next_id(start_id: u32) -> Result<u32, Errno> {
    let mut attr = bpf_attr();
    attr.get_id = bpf_get_id_arg {
        op:         bpf_get_op { start_id: start_id },
        next_id:    0,
        open_flags: 0,
    };
    bpf(BPF_PROG_GET_NEXT_ID, &attr)?;
    Ok(unsafe { attr.get_id.next_id })
}

pub fn bpf_map_get_fd_by_id(id: u32) -> Result<c_int, Errno> {
    let mut attr = bpf_attr();
    attr.get_id = bpf_get_id_arg {
        op:         bpf_get_op { map_id: id },
        next_id:    0,
        open_flags: 0,
    };
    bpf(BPF_MAP_GET_FD_BY_ID, &attr)
}

pub fn bpf_prog_get_fd_by_id(id: u32) -> Result<c_int, Errno> {
    let mut attr = bpf_attr();
    attr.get_id = bpf_get_id_arg {
        op:         bpf_get_op { prog_id: id },
        next_id:    0,
        open_flags: 0,
    };
    bpf(BPF_PROG_GET_FD_BY_ID, &attr)
}

pub fn bpf_obj_get_info_by_fd(fd: c_int, info: *mut c_void, len: u32) -> Result<(), Errno> {
    let mut attr = bpf_attr();
    attr.info = bpf_info_arg {
        bpf_fd:   fd   as u32,
        info_len: len,
        info:     info as u64,
    };
    bpf(BPF_OBJ_GET_INFO_BY_FD, &attr)?;
    Ok(())
}

fn bpf(cmd: bpf_cmd, attr: *const bpf_attr) -> Result<c_int, Errno> {
    let cmd  = cmd as c_int;
    let size = mem::size_of::<bpf_attr>() as c_uint;
    unsafe {
        match syscall(SYS_bpf, cmd, attr, size) {
            -1 => Err(errno()),
            rc => Ok(rc as c_int),
        }
    }
}

fn bpf_attr() -> bpf_attr {
    unsafe {
        mem::zeroed()
    }
}

pub fn close(fd: c_int) -> Result<(), Errno> {
    unsafe {
        match libc::close(fd) {
            0 => Ok(()),
            _ => Err(errno()),
        }
    }
}

pub fn name(s: &str) -> [u8; 16] {
    let mut dst = [0u8; 16];
    let src = s.bytes().take(dst.len() - 1);
    &dst.iter_mut().zip(src).for_each(|(d, s)| *d = s);
    dst
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Unsafe(..) => "unsafe BPF code",
            Other(..)  => "other error",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match self {
            Unsafe(..) => None,
            Other(..)  => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use std::fs;
    use std::io;
    use super::*;
    use ffi::bpf_map_type::*;
    use ffi::bpf_prog_type::*;

    fn create_map(kind: bpf_map_type, ksize: u32, vsize: u32, limit: u32) -> Result<c_int, Errno> {
        bpf_create_map(&bpf_map_create_arg {
            map_type:    kind as u32,
            key_size:    ksize,
            val_size:    vsize,
            max_entries: limit,
            .. Default::default()
        })
    }

    #[test]
    fn test_create_map() {
        assert_eq!(create_map(BPF_MAP_TYPE_UNSPEC, 4, 4, 16), Err(Errno(libc::EINVAL)));

        let kinds = &[
            BPF_MAP_TYPE_HASH,
            BPF_MAP_TYPE_ARRAY,
            BPF_MAP_TYPE_PROG_ARRAY,
            BPF_MAP_TYPE_PERF_EVENT_ARRAY,
        ];

        for kind in kinds {
            assert!(create_map(*kind, 4, 4, 16).is_ok());
        }
    }

    #[test]
    fn test_use_map() {
        let fd = create_map(BPF_MAP_TYPE_HASH, 4, 4, 16).unwrap();

        let key = 3u32;
        let val = 4u32;
        {
            let key = &key as *const _ as *const c_void;
            let val = &val as *const _ as *const c_void;
            assert!(bpf_update_elem(fd, key, val, 0).is_ok());
        }

        let mut val: u32 = 0;
        {
            let key = &key     as *const _ as *const c_void;
            let val = &mut val as *mut   _ as *mut   c_void;
            assert!(bpf_lookup_elem(fd, key, val).is_ok());
        }
        assert_eq!(val, 4);

        {
            let key = &key     as *const _ as *const c_void;
            let val = &mut val as *mut   _ as *mut   c_void;
            assert_eq!(bpf_delete_elem(fd, key),       Ok(()));
            assert_eq!(bpf_lookup_elem(fd, key, val),  Err(Errno(libc::ENOENT)));
        }
    }

    #[test]
    fn test_iter_map() {
        let fd = create_map(BPF_MAP_TYPE_ARRAY, 4, 4, 16).unwrap();

        let vs = &['A', 'B', 'C', 'D', 'E', 'F'];

        for (i, v) in vs.iter().enumerate() {
            let i = (i + 1) as u32;
            let key = &i as *const _ as *const c_void;
            let val =  v as *const _ as *const c_void;
            assert_eq!(bpf_update_elem(fd, key, val, 0), Ok(()));
        }

        let mut key  = 0u32;
        let mut val  = '\0';
        let mut next = 0u32;

        for (i, v) in vs.iter().enumerate() {
            let i = (i + 1) as u32;

            {
                let key  = &key      as *const _ as *const c_void;
                let next = &mut next as *mut   _ as *mut   c_void;
                let val  = &mut val  as *mut   _ as *mut   c_void;

                assert_eq!(bpf_get_next_key(fd, key, next), Ok(0));
                assert_eq!(bpf_lookup_elem(fd, next, val), Ok(()));
            }

            assert_eq!(next, i);
            assert_eq!(val, *v);

            key = next;
        }
    }

    #[test]
    fn test_prog_load() {
        let code = &[
            bpf_insn { code: 0xb7, regs: 0x00, off: 0x0000, imm: 0x00000000 }, // mov64 r0, 0x00
            bpf_insn { code: 0x95, regs: 0x00, off: 0x0000, imm: 0x00000000 }, // exit
        ];

        let license = CString::new("GPL").unwrap();
        let mut log = [0u8; 65535];

        let arg = bpf_prog_load_arg {
            prog_type: BPF_PROG_TYPE_SOCKET_FILTER as u32,
            insns:     code.as_ptr()               as u64,
            insn_cnt:  code.len()                  as u32,
            license:   license.as_ptr()            as u64,
            log_level: 2,
            .. Default::default()
        };

        assert!(bpf_prog_load(&arg, &mut log).is_ok());
    }

    #[test]
    fn test_pin_map() {
        let root = match bpf_fs() {
            Some(path) => path,
            None       => return,
        };

        let fd = create_map(BPF_MAP_TYPE_HASH, 4, 4, 16).unwrap();

        let key = 3u32;
        let val = 4u32;
        {
            let key = &key as *const _ as *const c_void;
            let val = &val as *const _ as *const c_void;
            assert_eq!(bpf_update_elem(fd, key, val, 0), Ok(()));
        }

        let path  = format!("{}/test_pin_map", root);
        let cpath = CString::new(path.clone()).unwrap();

        assert!(bpf_obj_pin(fd, cpath.as_ptr()).is_ok());

        let fd = bpf_obj_get(cpath.as_ptr()).unwrap();

        let mut val: u32 = 0;
        {
            let key = &key     as *const _ as *const c_void;
            let val = &mut val as *mut   _ as *mut   c_void;
            assert!(bpf_lookup_elem(fd, key, val).is_ok());
        }
        assert_eq!(val, 4);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_get_map_info() {
        let name = name("test");

        let fd = bpf_create_map(&bpf_map_create_arg {
            map_type:    BPF_MAP_TYPE_HASH as u32,
            key_size:    4,
            val_size:    4,
            max_entries: 16,
            map_name:    name,
            .. Default::default()
        }).unwrap();

        let mut info: bpf_map_info = unsafe { mem::zeroed() };
        {
            let size = mem::size_of_val(&info) as u32;
            let info = &mut info as *mut _ as *mut c_void;
            assert!(bpf_obj_get_info_by_fd(fd, info, size).is_ok());
        }

        assert_eq!(info._type,       BPF_MAP_TYPE_HASH as u32);
        assert_eq!(info.key_size,    4);
        assert_eq!(info.val_size,    4);
        assert_eq!(info.max_entries, 16);
        assert_eq!(info.map_flags,   0);
        assert_eq!(info.name,        name);
    }

    #[test]
    fn test_get_prog_info() {
        let code = &[
            bpf_insn { code: 0xb7, regs: 0x00, off: 0x0000, imm: 0x00000000 }, // mov64 r0, 0x00
            bpf_insn { code: 0x95, regs: 0x00, off: 0x0000, imm: 0x00000000 }, // exit
        ];

        let name    = name("test");
        let license = CString::new("GPL").unwrap();
        let mut log = [0u8; 65535];

        let arg = bpf_prog_load_arg {
            prog_type: BPF_PROG_TYPE_SOCKET_FILTER as u32,
            insns:     code.as_ptr()               as u64,
            insn_cnt:  code.len()                  as u32,
            license:   license.as_ptr()            as u64,
            prog_name: name,
            .. Default::default()
        };

        let fd = bpf_prog_load(&arg, &mut log).unwrap();

        let mut info: bpf_prog_info = unsafe { mem::zeroed() };
        {
            let size = mem::size_of_val(&info) as u32;
            let info = &mut info as *mut _ as *mut c_void;
            assert!(bpf_obj_get_info_by_fd(fd, info, size).is_ok());
        }

        assert_eq!(info._type, BPF_PROG_TYPE_SOCKET_FILTER as u32);
        assert_eq!(info.name,  name);
    }

    fn bpf_fs() -> Option<String> {
        use std::fs::File;
        use std::io::*;

        let f = File::open("/proc/mounts").unwrap();
        let r = BufReader::new(f);

        for line in r.lines() {
            let line = line.unwrap();
            let line = line.split(" ").collect::<Vec<_>>();
            if let &["bpf", path] = &line[0..2] {
                return Some(path.to_owned());
            }
        }

        write!(&mut io::stdout(), "BPF filesystem not mounted, skipping ").unwrap();

        None
    }
}
