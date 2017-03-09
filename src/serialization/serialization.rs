extern crate byteorder;

#[path = "tests.rs"]
#[cfg(test)]
mod tests;

#[path = "benchmarks.rs"]
#[cfg(all(test, feature = "bench_private"))]
mod benchmarks;

#[path = "v2_serializer.rs"]
mod v2_serializer;
pub use self::v2_serializer::{V2Serializer, SerializeError};

#[path = "deserializer.rs"]
mod deserializer;
pub use self::deserializer::{Deserializer, DeserializeError};

const V2_COOKIE_BASE: u32 = 0x1c849303;

const V2_COOKIE: u32 = V2_COOKIE_BASE | 0x10;

const V2_HEADER_SIZE: usize = 40;

