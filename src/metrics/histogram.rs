use std::fmt;
use std::iter;
use std::sync::{Arc, Weak};
use std::time::Instant;

use {Result, Metric, Collect, CollectorRegistry};
use default_registry;
use atomic::{AtomicF64, AtomicU64};
use bucket::{Bucket, CumulativeBuckets};
use label::{Label, Labels};
use metric::{MetricName, Help};
use timestamp::{self, Timestamp, TimestampMut};

#[derive(Debug, Clone)]
pub struct Histogram(Arc<Inner>);
impl Histogram {
    pub fn family_name(&self) -> &str {
        self.0.bucket_name.as_str()
    }
    pub fn bucket_name(&self) -> &str {
        self.0.bucket_name.as_str()
    }
    pub fn count_name(&self) -> &str {
        self.0.count_name.as_str()
    }
    pub fn sum_name(&self) -> &str {
        self.0.sum_name.as_str()
    }
    pub fn help(&self) -> Option<&str> {
        self.0.help.as_ref().map(|h| h.0.as_ref())
    }
    pub fn labels(&self) -> &Labels {
        &self.0.labels
    }
    // TODO: HistogramLabelsMut
    // pub fn labels_mut(&mut self) -> LabelsMut {
    //     LabelsMut::new(&self.0.labels)
    // }
    pub fn timetamp(&self) -> &Timestamp {
        &self.0.timestamp
    }
    pub fn timetamp_mut(&mut self) -> TimestampMut {
        TimestampMut(&self.0.timestamp)
    }
    pub fn buckets(&self) -> &[Bucket] {
        &self.0.buckets
    }
    pub fn cumulative_buckets(&self) -> CumulativeBuckets {
        CumulativeBuckets::new(&self.0.buckets)
    }
    pub fn observe(&mut self, value: f64) {
        assert_ne!(value, ::std::f64::NAN);
        let i = self.0
            .buckets
            .binary_search_by(|b| {
                b.upper_bound().partial_cmp(&value).expect("Never fails")
            })
            .unwrap_or_else(|i| i);
        self.0.buckets.get(i).map(|b| b.increment());
        self.0.count.inc();
        self.0.sum.update(|v| v + value);
    }
    pub fn count(&self) -> u64 {
        self.0.count.get()
    }
    pub fn sum(&self) -> f64 {
        self.0.sum.get()
    }
    pub fn time<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let elapsed = timestamp::duration_to_unixtime_seconds(start.elapsed());
        self.observe(elapsed);
        result
    }
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

        for bucket in self.buckets() {
            write!(
                f,
                "{}_bucket{{le=\"{}\"",
                self.family_name(),
                bucket.upper_bound()
            )?;
            for label in self.labels().iter() {
                write!(f, ",{}={:?}", label.name(), label.value())?;
            }
            writeln!(f, "}} {}{}", bucket.count(), timestamp)?;
        }
        writeln!(
            f,
            "{}_sum{} {}{}",
            self.family_name(),
            labels,
            self.sum(),
            timestamp
        )?;
        write!(
            f,
            "{}_count{} {}{}",
            self.family_name(),
            labels,
            self.count(),
            timestamp
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct HistogramBuilder {
    namespace: Option<String>,
    subsystem: Option<String>,
    name: String,
    help: Option<String>,
    labels: Vec<(String, String)>,
    buckets: Vec<Bucket>,
    registries: Vec<CollectorRegistry>,
}
impl HistogramBuilder {
    pub fn new(name: &str) -> Self {
        HistogramBuilder {
            namespace: None,
            subsystem: None,
            name: name.to_string(),
            help: None,
            labels: Vec::new(),
            buckets: Vec::new(),
            registries: Vec::new(),
        }
    }
    pub fn with_linear_buckets(name: &str, start: f64, width: f64, count: usize) -> Self {
        let mut this = Self::new(name);
        for x in (0..count).map(|i| start + i as f64 * width) {
            this.bucket(x);
        }
        this
    }
    pub fn with_exponential_buckets(name: &str, start: f64, factor: f64, count: usize) -> Self {
        let mut this = Self::new(name);
        for x in (0..count).map(|i| start + factor.powi(i as i32)) {
            this.bucket(x);
        }
        this
    }
    pub fn bucket(&mut self, upper_bound: f64) -> &mut Self {
        self.buckets.push(Bucket::new(upper_bound).expect("TODO"));
        self
    }
    pub fn namespace(&mut self, namespace: &str) -> &mut Self {
        self.namespace = Some(namespace.to_string());
        self
    }
    pub fn subsystem(&mut self, subsystem: &str) -> &mut Self {
        self.subsystem = Some(subsystem.to_string());
        self
    }
    pub fn help(&mut self, help: &str) -> &mut Self {
        self.help = Some(help.to_string());
        self
    }
    pub fn label(&mut self, name: &str, value: &str) -> &mut Self {
        assert_ne!(name, "le"); // TODO: validate in `finish` method
        self.labels.retain(|l| l.0 != name);
        self.labels.push((name.to_string(), value.to_string()));
        self.labels.sort();
        self
    }
    pub fn registry(&mut self, registry: CollectorRegistry) -> &mut Self {
        self.registries.push(registry);
        self
    }
    pub fn default_registry(&mut self) -> &mut Self {
        self.registry(default_registry())
    }
    pub fn finish(&self) -> Result<Histogram> {
        let namespace = self.namespace.as_ref().map(AsRef::as_ref);
        let subsystem = self.subsystem.as_ref().map(AsRef::as_ref);
        let bucket_name = track!(MetricName::new(namespace, subsystem, &self.name, None))?;
        let count_name = track!(MetricName::new(
            namespace,
            subsystem,
            &self.name,
            Some("count"),
        ))?;
        let sum_name = track!(MetricName::new(
            namespace,
            subsystem,
            &self.name,
            Some("sum"),
        ))?;
        let labels = track!(
            self.labels
                .iter()
                .map(|&(ref name, ref value)| track!(Label::new(name, value)))
                .collect::<Result<_>>()
        )?;
        let mut buckets = self.buckets
            .iter()
            .map(|b| Bucket::new(b.upper_bound()).expect("TODO"))
            .collect::<Vec<_>>();
        buckets.sort_by(|a, b| {
            a.upper_bound().partial_cmp(&b.upper_bound()).expect(
                "Never fails",
            )
        });
        let inner = Inner {
            bucket_name,
            count_name,
            sum_name,
            labels: Labels::new(labels),
            help: self.help.clone().map(Help),
            timestamp: Timestamp::new(),
            buckets,
            count: AtomicU64::new(0),
            sum: AtomicF64::new(0.0),
        };
        let histogram = Histogram(Arc::new(inner));
        for r in self.registries.iter() {
            track!(r.register(histogram.collector()))?;
        }
        Ok(histogram)
    }
}

#[derive(Debug, Clone)]
pub struct HistogramCollector(Weak<Inner>);
impl Collect for HistogramCollector {
    type Metrics = iter::Once<Metric>;
    fn collect(&mut self) -> Option<Self::Metrics> {
        self.0.upgrade().map(|inner| {
            iter::once(Metric::Histogram(Histogram(inner)))
        })
    }
}

#[derive(Debug)]
struct Inner {
    bucket_name: MetricName,
    count_name: MetricName,
    sum_name: MetricName,
    labels: Labels,
    help: Option<Help>,
    timestamp: Timestamp,
    buckets: Vec<Bucket>,
    count: AtomicU64,
    sum: AtomicF64,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        let mut histogram =
            track_try_unwrap!(HistogramBuilder::with_linear_buckets("foo", 0.0, 10.0, 5).finish());
        assert_eq!(histogram.bucket_name(), "foo");
        assert_eq!(histogram.count_name(), "foo_count");
        assert_eq!(histogram.sum_name(), "foo_sum");

        histogram.observe(7.0);
        histogram.observe(12.0);
        histogram.observe(50.1);
        histogram.observe(10.0);
        assert_eq!(
            histogram.cumulative_buckets().collect::<Vec<_>>(),
            [(0.0, 0), (10.0, 2), (20.0, 3), (30.0, 3), (40.0, 3)]
        );
        assert_eq!(histogram.count(), 4);
        assert_eq!(histogram.sum(), 79.1);

        assert_eq!(
            histogram.to_string(),
            r#"foo_bucket{le="0"} 0
foo_bucket{le="10"} 2
foo_bucket{le="20"} 1
foo_bucket{le="30"} 0
foo_bucket{le="40"} 0
foo_sum 79.1
foo_count 4"#
        );
    }
}
