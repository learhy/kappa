# ebpf - Linux eBPF interface

ebpf provides a Rust interface to the Linux extended BPF subsystem
including support for loading eBPF programs from ELF object files,
attaching them to various points in the kernel, exchanging data via
BPF maps, etc.

ebpf also provides an interface to AF_XDP sockets for fast packet
RX and TX.

## License

Copyright (C) 2017 - Will Glozer

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License version 3 as
published by the Free Software Foundation.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
