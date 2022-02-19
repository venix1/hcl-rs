#![doc = include_str!("../README.md")]
#![allow(clippy::should_implement_trait)]
#![warn(missing_docs)]

pub mod de;
mod de_v2;
pub mod error;
mod number;
mod parser;
#[allow(missing_docs)]
pub mod structure;
pub mod value;

pub use de::{from_reader, from_str};
pub use error::{Error, Result};
pub use number::Number;
pub use parser::parse;
pub use value::{Map, Value};
