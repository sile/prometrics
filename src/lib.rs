#[macro_use]
extern crate trackable;

pub use error::{Error, ErrorKind};

pub mod collector;
pub mod format;
pub mod metric;
pub mod registry;

mod error;

pub type Result<T> = std::result::Result<T, Error>;
