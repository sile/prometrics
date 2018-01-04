use std;
use std::fmt;
use std::iter;
use std::sync::{Arc, Weak};
use std::time::Instant;

use {Collect, ErrorKind, Registry, Result};
use default_registry;
use atomic::{AtomicF64, AtomicU64};
use bucket::{Bucket, CumulativeBuckets};
use label::{Label, Labels, LabelsMut};
use metric::{Metric, MetricName, MetricValue};
use timestamp::{self, Timestamp, TimestampMut};

/// `Histogram` samples observations (usually things like request durations or response sizes) and
/// counts them in configurable buckets.
/// It also provides a sum of all observed values.
///
/// Cloned histograms share the same buckets.
#[derive(Debug, Clone)]
pub struct Histogram(Arc<Inner>);
impl Histogram {
    /// Makes a new `Histogram` instance.
    ///
    /// Note that it is recommended to create this via `HistogramBuilder`.
    pub fn new(name: &str) -> Result<Self> {
        HistogramBuilder::new(name).finish()
    }

    /// Returns the name of this histogram.
    pub fn metric_name(&self) -> &MetricName {
        &self.0.bucket_name
    }

    /// Returns the help of this histogram.
    pub fn help(&self) -> Option<&str> {
        self.0.help.as_ref().map(|h| h.as_ref())
    }

    /// Returns the user defined labels of this histogram.
    pub fn labels(&self) -> &Labels {
        &self.0.labels
    }

    /// Returns the mutable user defined labels of this histogram.
    pub fn labels_mut(&mut self) -> LabelsMut {
        LabelsMut::new(&self.0.labels, Some("le"))
    }

    /// Returns the timestamp of this histogram.
    pub fn timetamp(&self) -> &Timestamp {
        &self.0.timestamp
    }

    /// Returns the mutable timestamp of this histogram.
    pub fn timetamp_mut(&mut self) -> TimestampMut {
        TimestampMut::new(&self.0.timestamp)
    }

    /// Returns the buckets of this histogram.
    pub fn buckets(&self) -> &[Bucket] {
        &self.0.buckets
    }

    /// Returns the cumulative buckets of this histogram.
    pub fn cumulative_buckets(&self) -> CumulativeBuckets {
        CumulativeBuckets::new(&self.0.buckets)
    }

    /// Returns the total observation count.
    pub fn count(&self) -> u64 {
        self.0.count.get()
    }

    /// Returns the sum of the observed values.
    pub fn sum(&self) -> f64 {
        self.0.sum.get()
    }

    /// Observes a value.
    pub fn observe(&self, value: f64) {
        assert!(!value.is_nan());
        let i = self.0
            .buckets
            .binary_search_by(|b| b.upper_bound().partial_cmp(&value).expect("Never fails"))
            .unwrap_or_else(|i| i);
        self.0.buckets.get(i).map(|b| b.increment());
        self.0.count.inc();
        self.0.sum.update(|v| v + value);
    }

    /// Measures the exeuction time of `f` and observes its duration in seconds.
    pub fn time<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let elapsed = timestamp::duration_to_unixtime_seconds(start.elapsed());
        self.observe(elapsed);
        result
    }

    /// Returns a collector for this histogram.
    pub fn collector(&self) -> HistogramCollector {
        HistogramCollector(Arc::downgrade(&self.0))
    }
}
impl fmt::Display for Histogram {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let labels = if !self.labels().is_empty() {
            self.labels().to_string()
        } else {
            "".to_string()
        };
        let timestamp = if let Some(t) = self.timetamp().get() {
            format!(" {}", t)
        } else {
            "".to_string()
        };

        for bucket in self.cumulative_buckets() {
            write!(
                f,
                "{}_bucket{{le=\"{}\"",
                self.metric_name(),
                MetricValue(bucket.upper_bound())
            )?;
            for label in self.labels().iter() {
                write!(f, ",{}={:?}", label.name(), label.value())?;
            }
            writeln!(f, "}} {}{}", bucket.cumulative_count(), timestamp)?;
        }
        writeln!(
            f,
            "{}_sum{} {}{}",
            self.metric_name(),
            labels,
            MetricValue(self.sum()),
            timestamp
        )?;
        write!(
            f,
            "{}_count{} {}{}",
            self.metric_name(),
            labels,
            self.count(),
            timestamp
        )?;
        Ok(())
    }
}

/// `Histogram` builder.
#[derive(Debug)]
pub struct HistogramBuilder {
    namespace: Option<String>,
    subsystem: Option<String>,
    name: String,
    help: Option<String>,
    labels: Vec<(String, String)>,
    bucket_upper_bounds: Vec<f64>,
    registries: Vec<Registry>,
}
impl HistogramBuilder {
    /// Makes a builder for histograms named `name`.
    pub fn new(name: &str) -> Self {
        HistogramBuilder {
            namespace: None,
            subsystem: None,
            name: name.to_string(),
            help: None,
            labels: Vec::new(),
            bucket_upper_bounds: vec![std::f64::INFINITY],
            registries: Vec::new(),
        }
    }

    /// Makes a builder with the specified linear buckets.
    pub fn with_linear_buckets(name: &str, start: f64, width: f64, count: usize) -> Self {
        let mut this = Self::new(name);
        for x in (0..count).map(|i| start + i as f64 * width) {
            this.bucket(x);
        }
        this
    }

    /// Makes a builder with the specified exponential buckets.
    pub fn with_exponential_buckets(name: &str, start: f64, factor: f64, count: usize) -> Self {
        let mut this = Self::new(name);
        for x in (0..count).map(|i| start + factor.powi(i as i32)) {
            this.bucket(x);
        }
        this
    }

    /// Sets the namespace part of the metric name of this.
    pub fn namespace(&mut self, namespace: &str) -> &mut Self {
        self.namespace = Some(namespace.to_string());
        self
    }

    /// Sets the subsystem part of the metric name of this.
    pub fn subsystem(&mut self, subsystem: &str) -> &mut Self {
        self.subsystem = Some(subsystem.to_string());
        self
    }

    /// Sets the help of this.
    pub fn help(&mut self, help: &str) -> &mut Self {
        self.help = Some(help.to_string());
        self
    }

    /// Adds a label.
    ///
    /// Note that `name` will be validated in the invocation of the `finish` method.
    ///
    /// The name `"le"` is reserved for designating buckets.
    pub fn label(&mut self, name: &str, value: &str) -> &mut Self {
        self.labels.retain(|l| l.0 != name);
        self.labels.push((name.to_string(), value.to_string()));
        self.labels.sort();
        self
    }

    /// Adds a registry to which the resulting histograms will be registered..
    pub fn registry(&mut self, registry: Registry) -> &mut Self {
        self.registries.push(registry);
        self
    }

    /// Adds the default registry.
    pub fn default_registry(&mut self) -> &mut Self {
        self.registry(default_registry())
    }

    /// Adds a bucket.
    pub fn bucket(&mut self, upper_bound: f64) -> &mut Self {
        self.bucket_upper_bounds.push(upper_bound);
        self
    }

    /// Builds a histogram.
    ///
    /// # Errors
    ///
    /// This method will return `Err(_)` if one of the following conditions is satisfied:
    ///
    /// - Any of the name of the metric or labels is malformed
    /// - There is a bucket whose upper bound is `NaN`
    pub fn finish(&self) -> Result<Histogram> {
        let namespace = self.namespace.as_ref().map(AsRef::as_ref);
        let subsystem = self.subsystem.as_ref().map(AsRef::as_ref);
        let bucket_name = track!(MetricName::new(namespace, subsystem, &self.name))?;
        let labels = track!(
            self.labels
                .iter()
                .map(|&(ref name, ref value)| {
                    track_assert_ne!(name, "le", ErrorKind::InvalidInput);
                    track!(Label::new(name, value))
                })
                .collect::<Result<_>>()
        )?;
        let mut buckets = track!(
            self.bucket_upper_bounds
                .iter()
                .map(|upper_bound| track!(Bucket::new(*upper_bound)))
                .collect::<Result<Vec<_>>>()
        )?;
        buckets.sort_by(|a, b| {
            a.upper_bound()
                .partial_cmp(&b.upper_bound())
                .expect("Never fails")
        });
        let inner = Inner {
            bucket_name,
            labels: Labels::new(labels),
            help: self.help.clone(),
            timestamp: Timestamp::new(),
            buckets,
            count: AtomicU64::new(0),
            sum: AtomicF64::new(0.0),
        };
        let histogram = Histogram(Arc::new(inner));
        for r in &self.registries {
            r.register(histogram.collector());
        }
        Ok(histogram)
    }
}

/// `Collect` trait implmentation for `Histogram`.
#[derive(Debug, Clone)]
pub struct HistogramCollector(Weak<Inner>);
impl Collect for HistogramCollector {
    type Metrics = iter::Once<Metric>;
    fn collect(&mut self) -> Option<Self::Metrics> {
        self.0
            .upgrade()
            .map(|inner| iter::once(Metric::Histogram(Histogram(inner))))
    }
}

#[derive(Debug)]
struct Inner {
    bucket_name: MetricName,
    labels: Labels,
    help: Option<String>,
    timestamp: Timestamp,
    buckets: Vec<Bucket>,
    count: AtomicU64,
    sum: AtomicF64,
}

#[cfg(test)]
mod test {
    use std::f64::INFINITY;
    use super::*;

    #[test]
    fn it_works() {
        let histogram =
            track_try_unwrap!(HistogramBuilder::with_linear_buckets("foo", 0.0, 10.0, 5).finish());
        assert_eq!(histogram.metric_name().to_string(), "foo");

        histogram.observe(7.0);
        histogram.observe(12.0);
        histogram.observe(50.1);
        histogram.observe(10.0);
        assert_eq!(
            histogram
                .cumulative_buckets()
                .map(|b| (b.upper_bound(), b.cumulative_count()))
                .collect::<Vec<_>>(),
            [
                (0.0, 0),
                (10.0, 2),
                (20.0, 3),
                (30.0, 3),
                (40.0, 3),
                (INFINITY, 4),
            ]
        );
        assert_eq!(histogram.count(), 4);
        assert_eq!(histogram.sum(), 79.1);

        assert_eq!(
            histogram.to_string(),
            r#"foo_bucket{le="0"} 0
foo_bucket{le="10"} 2
foo_bucket{le="20"} 3
foo_bucket{le="30"} 3
foo_bucket{le="40"} 3
foo_bucket{le="+Inf"} 4
foo_sum 79.1
foo_count 4"#
        );
    }
}
