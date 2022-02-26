#![doc = include_str!("../README.md")]
#![allow(clippy::should_implement_trait)]
#![warn(missing_docs)]

pub mod de;
pub mod error;
mod number;
mod parser;
#[allow(missing_docs)]
pub mod ser;
#[allow(missing_docs)]
mod structure;
pub mod value;

pub use de::{from_reader, from_str, Deserializer};
pub use error::{Error, Result};
pub use number::Number;
pub use parser::parse;
pub use ser::{to_string, to_vec, to_writer, Serializer};
pub use structure::{Attribute, Block, BlockLabel, Body, Structure};
pub use value::{Map, Value};
