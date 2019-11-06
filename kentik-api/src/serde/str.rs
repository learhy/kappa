use std::fmt::Display;
use std::str::FromStr;
use serde::Deserialize;
use serde::de::{self, Deserializer};
use serde::ser::Serializer;

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where T: FromStr,
          T::Err: Display,
          D: Deserializer<'de>
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}

pub fn serialize<T: Display, S: Serializer>(v: &T, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&v.to_string())
}
