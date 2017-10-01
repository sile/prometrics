extern crate atomic_immut;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate trackable;

pub use collector::Collector;
pub use error::{Error, ErrorKind};
pub use metric::{Metric, MetricFamily, MetricKind};
pub use registry::{default_registry, default_gatherer, CollectorRegistry, MetricsGatherer};

pub mod bucket;
pub mod label;
pub mod metrics;
pub mod timestamp;

mod atomic;
mod collector;
mod error;
mod metric;
mod registry;

pub type Result<T> = std::result::Result<T, Error>;
