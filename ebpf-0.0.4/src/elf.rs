// Copyright (C) 2017 - Will Glozer. All rights reserved.

use std::error;
use std::ffi::CString;
use std::rc::Rc;
use std::fmt;
use std::mem;
use byteorder::{ByteOrder, LE};
use errno::Errno;
use xmas_elf::ElfFile;
use xmas_elf::sections::SectionHeader;
use xmas_elf::sections::SectionData::*;
use xmas_elf::sections::ShType::*;
use xmas_elf::symbol_table::Entry;
use xmas_elf::symbol_table::Binding::*;
use zero::read_array;
use bpf::{self, Kind, Program};
use ffi::*;
use sys;
use self::Error::*;
use self::Item::*;
use self::Kind::*;

#[derive(Debug)]
pub struct Loader {
    pub code:    Vec<Code>,
    pub maps:    Vec<Map>,
    pub rels:    Vec<Relocation>,
    pub symbols: Vec<Symbol>,
    pub license: CString,
    pub version: u32,
}

pub struct Code {
    pub symbol: Symbol,
    pub kind:   Kind,
    pub code:   Vec<bpf_insn>,
}

#[derive(Debug)]
pub struct Map {
    pub symbol: Symbol,
    pub create: bpf_map_create_arg,
}

#[derive(Debug)]
pub struct Relocation {
    pub section: u16,
    pub offset:  usize,
    pub symbol:  Symbol,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Symbol {
    pub section: u16,
    pub name:    String,
    pub value:   u64,
}

#[derive(Debug)]
pub enum Error {
    ELF(&'static str),
    Invalid(Item),
    Missing(Item),
    Syscall(Errno),
    Program(sys::Error),
}

#[derive(Debug)]
pub enum Item {
    License,
    Map,
    Version,
    Symbol,
}

impl Loader {
    pub fn new(bytes: &[u8]) -> Result<Self, Error> {
        let elf = ElfFile::new(bytes)?;

        let mut loader = Loader {
            code:    Vec::new(),
            maps:    Vec::new(),
            rels:    Vec::new(),
            symbols: Vec::new(),
            license: CString::default(),
            version: 0,
        };

        for (index, sec) in elf.section_iter().enumerate() {
            let kind = sec.get_type()?;
            let name = sec.get_name(&elf);
            let data = match sec.size() {
                n if n > 0 => sec.raw_data(&elf),
                _          => &[],
            };

            match (kind, name) {
                (ProgBits, Ok("maps"),  ) => loader.maps.extend(maps(data, &elf, index)?),
                (ProgBits, Ok("license")) => loader.license = license(data)?,
                (ProgBits, Ok("version")) => loader.version = version(data)?,
                (ProgBits, Ok(name),    ) => loader.code.extend(code(name, &elf, index)?),
                (Rel,      _,           ) => loader.rels.extend(relocations(sec, &elf)?),
                _                         => (),
            };

            loader.symbols.extend(symbols(&elf, index as u16)?);
        }

        Ok(loader)
    }

    pub fn load(&mut self) -> Result<Vec<Program>, Error> {
        let rels = &self.rels;
        let maps = self.maps.iter().flat_map(|map| {
            let rels: Vec<_> = rels.iter().filter(|r| r.symbol == map.symbol).collect();

            let arg = bpf_map_create_arg {
                map_type:    map.create.map_type,
                key_size:    map.create.key_size,
                val_size:    map.create.val_size,
                max_entries: map.create.max_entries,
                map_flags:   0,
                .. Default::default()
            };

            rels.first().cloned().map(|_| {
                let fd = sys::bpf_create_map(&arg)?;
                Ok((Rc::new(bpf::Map {
                    name:  map.symbol.name.to_owned(),
                    fd:    fd,
                    ksize: arg.key_size    as usize,
                    vsize: arg.val_size    as usize,
                    limit: arg.max_entries as usize,
                }), rels))
            })
        }).collect::<Result<Vec<_>, Error>>()?;

        let license = &self.license;
        let version = self.version;
        let mut log = [0u8; 65535];

        self.code.iter_mut().map(|ref mut code| {
            let section = code.symbol.section;
            let name    = code.symbol.name.clone();
            let kind    = code.kind.clone();
            let code    = &mut code.code;

            let maps = maps.iter().flat_map(|&(ref map, ref rels)| {
                let mut maps = rels.iter().filter(|r| r.section == section).map(|rel| {
                    let offset = rel.offset / mem::size_of::<bpf_insn>();
                    let insn   = &mut code[offset as usize];

                    insn.regs |= BPF_PSEUDO_MAP_FD << 4;
                    insn.imm   = map.fd;

                    Rc::clone(map)
                }).collect::<Vec<_>>();

                maps.dedup_by_key(|map| map.fd);

                maps
            }).collect::<Vec<_>>();

            let arg = bpf_prog_load_arg {
                prog_type:    prog_type(&kind) as u32,
                insns:        code.as_ptr()    as u64,
                insn_cnt:     code.len()       as u32,
                license:      license.as_ptr() as u64,
                kern_version: version          as u32,
                .. Default::default()
            };

            let fd = sys::bpf_prog_load(&arg, &mut log)?;

            Ok(Program{ name, kind, fd, maps })
        }).collect()
    }
}

fn relocations(sec: SectionHeader, elf: &ElfFile) -> Result<Vec<Relocation>, Error> {
    let symtab  = sec.link() as u16;
    let section = sec.info() as u16;

    let rel = |offset: usize, symbol: u32| {
        Ok(Relocation {
            section: section,
            offset:  offset,
            symbol:  resolve(elf, symtab, symbol as usize)?,
        })
    };

    match sec.get_data(elf)? {
        Rel32(rs) => rs.iter().map(|r| rel(r.get_offset() as usize, r.get_symbol_table_index())).collect(),
        Rel64(rs) => rs.iter().map(|r| rel(r.get_offset() as usize, r.get_symbol_table_index())).collect(),
        _         => unreachable!(),
    }
}

fn code(name: &str, elf: &ElfFile, index: usize) -> Result<Option<Code>, Error> {
    let index = index as u16;
    let sec   = elf.section_header(index)?;
    let syms  = symbols(elf, index)?;
    let data  = sec.raw_data(&elf);

    let code = |kind| {
        syms.first().map(|sym| {
            Code {
                symbol: sym.clone(),
                kind:   kind,
                code:   read_array(data).to_vec(),
            }
        })
    };

    let mut split = name.splitn(2, '/');
    let code = match (split.next(), split.next()) {
        (Some("kprobe"),     Some(event)) => code(Kprobe(event.into())),
        (Some("kretprobe"),  Some(event)) => code(Kretprobe(event.into())),
        (Some("tracepoint"), Some(event)) => code(Tracepoint(event.into())),
        (Some("xdp"),        Some(name))  => code(XDP(name.into())),
        _                                 => None,
    };

    Ok(code)
}

fn maps(data: &[u8], elf: &ElfFile, index: usize) -> Result<Vec<Map>, Error> {
    let args = read_array(data);
    symbols(elf, index as u16)?.iter().map(|sym| {
        let size   = mem::size_of::<bpf_map_create_arg>();
        let offset = sym.value as usize / size;
        let create = *args.get(offset).ok_or(Missing(Map))?;
        let symbol = sym.clone();
        Ok(Map { create, symbol })
    }).collect()
}

fn license(data: &[u8]) -> Result<CString, Error> {
    let n    = data.len() - 1;
    let data = data[..n].to_vec();
    CString::new(data).map_err(|_| Invalid(License))
}

fn version(data: &[u8]) -> Result<u32, Error> {
    match data.len() {
        4 => Ok(LE::read_u32(data)),
        _ => Err(Invalid(Version)),
    }
}

fn resolve(elf: &ElfFile, section: u16, index: usize) -> Result<Symbol, Error> {
    let symtab = match elf.header.pt2.sh_count() {
        n if n > section => elf.section_header(section)?,
        _                => return Err(Missing(Symbol)),
    };

    match symtab.get_data(elf)? {
        SymbolTable32(entries) => entries.get(index).map(|e| sym(e, elf)),
        SymbolTable64(entries) => entries.get(index).map(|e| sym(e, elf)),
        _                      => None,
    }.unwrap_or(Err(Missing(Symbol)))
}

fn symbols(elf: &ElfFile, section: u16) -> Result<Vec<Symbol>, Error> {
    let filter = |e: &dyn Entry| {
        match (e.get_binding(), e.shndx() == section) {
            (Ok(Global), true) => Some(sym(e, elf)),
            _                  => None,
        }
    };

    let mut syms = Vec::new();
    for sec in elf.section_iter().filter(|s| s.get_type() == Ok(SymTab)) {
        match sec.get_data(elf)? {
            SymbolTable32(entries) => syms.extend(entries.iter().flat_map(|e| filter(e))),
            SymbolTable64(entries) => syms.extend(entries.iter().flat_map(|e| filter(e))),
            _                      => (),
        }
    }

    syms.into_iter().collect()
}

fn sym(e: &dyn Entry, elf: &ElfFile) -> Result<Symbol, Error> {
    Ok(Symbol {
        section: e.shndx(),
        name:    e.get_name(elf)?.into(),
        value:   e.value(),
    })
}

fn prog_type(kind: &Kind) -> bpf_prog_type {
    use ffi::bpf_prog_type::*;
    match *kind {
        Kprobe(..)     => BPF_PROG_TYPE_KPROBE,
        Kretprobe(..)  => BPF_PROG_TYPE_KPROBE,
        Socket         => BPF_PROG_TYPE_SOCKET_FILTER,
        Tracepoint(..) => BPF_PROG_TYPE_TRACEPOINT,
        XDP(..)        => BPF_PROG_TYPE_XDP,
    }
}

impl From<&'static str> for Error {
    fn from(err: &'static str) -> Self {
        ELF(err)
    }
}

impl From<Errno> for Error {
    fn from(err: Errno) -> Self {
        Syscall(err)
    }
}

impl From<sys::Error> for Error {
    fn from(err: sys::Error) -> Self {
        Program(err)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            ELF(..)     => "ELF loader error",
            Invalid(..) => "invalid item",
            Missing(..) => "missing item",
            Syscall(..) => "syscall error",
            Program(..) => "program error",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match self {
            ELF(..)     => None,
            Invalid(..) => None,
            Missing(..) => None,
            Syscall(..) => None,
            Program(e)  => Some(e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let count = format!("[{} instructions]", self.code.len());
        f.debug_struct("Code")
            .field("kind", &self.kind)
            .field("code", &count)
            .finish()

    }
}
