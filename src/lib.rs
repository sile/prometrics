extern crate atomic_immut;
#[macro_use]
extern crate trackable;

pub use error::{Error, ErrorKind};
pub use metric::Metric;

// pub mod collector;
// pub mod format;
// pub mod metric;
pub mod metrics;
// pub mod registry;
pub mod types;

mod error;
mod metric;

pub type Result<T> = std::result::Result<T, Error>;
