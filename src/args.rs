use std::fs;
use std::str::FromStr;
use anyhow::Result;

pub fn opt<T: FromStr>(arg: Option<&str>) -> Result<Option<T>> {
    Ok(arg.map(|s| T::from_str(s).map_err(|_| {
        let msg  = format!("invalid argument value '{}'", s);
        let kind = clap::ErrorKind::InvalidValue;
        clap::Error::with_description(&msg, kind)
    })).transpose()?)
}

pub fn read(path: String) -> Result<Vec<u8>> {
    Ok(fs::read(&path).map_err(|e| {
        let msg  = format!("invalid argument '{}': {}", path, e);
        let kind = clap::ErrorKind::InvalidValue;
        clap::Error::with_description(&msg, kind)
    })?)
}
