//! Client library for exposing [prometheus][prometheus] metrics.
//!
//! [prometheus]: https://prometheus.io/
//!
//! # References
//!
//! - [Data model](https://prometheus.io/docs/concepts/data_model/)
//! - [Metric types](https://prometheus.io/docs/concepts/metric_types/)
//! - [Writing client libraries](https://prometheus.io/docs/instrumenting/writing_clientlibs/)
//! - [Exposition formats](https://prometheus.io/docs/instrumenting/exposition_formats/)
#![warn(missing_docs)]
extern crate atomic_immut;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate trackable;

pub use collect::Collect;
pub use error::{Error, ErrorKind};
pub use metric::{Metric, MetricFamily, MetricKind};
pub use registry::{default_registry, default_gatherer, CollectorRegistry, MetricsGatherer};

pub mod bucket;
pub mod label;
pub mod metrics;
pub mod timestamp;

mod atomic;
mod collect;
mod error;
mod metric;
mod registry;

/// This crate specific `Result` type.
pub type Result<T> = std::result::Result<T, Error>;
