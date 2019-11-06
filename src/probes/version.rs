use nixv::Version;

pub trait LinuxVersionCode {
    fn decode(code: u32) -> Self;
    fn encode(&self)     -> u32;
}

impl LinuxVersionCode for Version {
    fn decode(code: u32) -> Self {
        let major = code as u64 >> 16 & 0xFF;
        let minor = code as u64 >>  8 & 0xFF;
        let patch = code as u64 >>  0 & 0xFF;
        Version::new(major, minor, patch)
    }

    fn encode(&self) -> u32 {
        let major = self.major as u32;
        let minor = self.minor as u32;
        let patch = self.patch as u32;
        major << 16 | minor << 8 | patch
    }
}
