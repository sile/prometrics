//! Client library for exposing [prometheus][prometheus] metrics.
//!
//! Currently this crate only support text format for exposing metrics.
//!
//! [prometheus]: https://prometheus.io/
//!
//! # Examples
//!
//! ```
//! use prometrics::default_gatherer;
//! use prometrics::metrics::{CounterBuilder, GaugeBuilder};
//!
//! let counter = CounterBuilder::new("count")
//!     .default_registry()
//!     .finish()
//!     .unwrap();
//! let gauge = GaugeBuilder::new("gauge")
//!     .label("foo", "bar")
//!     .default_registry()
//!     .finish()
//!     .unwrap();
//!
//!  counter.increment();
//!  gauge.set(12.3);
//!
//!  let metrics = default_gatherer().lock().unwrap().gather();
//!  assert_eq!(
//!     metrics.to_text(),
//!     format!("{}\n{}\n{}\n{}\n",
//!             "# TYPE count counter",
//!             "count 1",
//!             "# TYPE gauge gauge",
//!             "gauge{foo=\"bar\"} 12.3"));
//! ```
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
#[cfg(target_os = "linux")]
extern crate libc;
#[cfg(target_os = "linux")]
extern crate procinfo;
#[macro_use]
extern crate trackable;

pub use collect::Collect;
pub use error::{Error, ErrorKind};
pub use registry::{default_gatherer, default_registry, Gatherer, Registry};

pub mod bucket;
pub mod label;
pub mod metric;
pub mod metrics;
pub mod quantile;
pub mod timestamp;

mod aggregated_metrics;
mod atomic;
mod collect;
mod error;
mod registry;

/// This crate specific `Result` type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use super::*;
    use metrics::{CounterBuilder, GaugeBuilder, MetricBuilder};
    use registry::Gatherer;

    #[test]
    fn it_works() {
        let counter = CounterBuilder::new("count")
            .default_registry()
            .finish()
            .unwrap();
        let gauge = GaugeBuilder::new("gauge")
            .label("foo", "bar")
            .default_registry()
            .finish()
            .unwrap();

        counter.increment();
        gauge.set(12.3);

        let metrics = default_gatherer().lock().unwrap().gather();
        assert_eq!(
            format!("\n{}", metrics.to_text()),
            r#"
# TYPE count counter
count 1
# TYPE gauge gauge
gauge{foo="bar"} 12.3
"#
        );
    }

    #[test]
    fn aggregation_works() {
        let mut gatherer = Gatherer::new();
        let mut builder = MetricBuilder::new();
        builder.set_registry(gatherer.registry());

        let counter = builder.counter("count").finish().unwrap();
        let gauge = builder.gauge("gauge").label("foo", "bar").finish().unwrap();
        let histogram = builder.histogram("histogram").bucket(5.0).finish().unwrap();
        counter.increment();
        gauge.set(12.3);
        histogram.observe(4.0);

        let counter = builder.counter("count").finish().unwrap();
        let gauge = builder.gauge("gauge").label("foo", "bar").finish().unwrap();
        let histogram = builder
            .histogram("histogram")
            .bucket(10.0)
            .finish()
            .unwrap();
        counter.increment();
        gauge.set(12.3);
        histogram.observe(8.0);

        let gauge = builder.gauge("gauge").finish().unwrap();
        gauge.set(12.3);

        let metrics = gatherer.gather();
        assert_eq!(
            format!("\n{}", metrics.to_text()),
            r#"
# TYPE count counter
count 2
# TYPE gauge gauge
gauge 12.3
gauge{foo="bar"} 24.6
# TYPE histogram histogram
histogram_bucket{le="5"} 1
histogram_bucket{le="10"} 2
histogram_bucket{le="+Inf"} 2
histogram_sum 12
histogram_count 2
"#
        );
    }
}
