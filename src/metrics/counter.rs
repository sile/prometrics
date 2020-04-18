use std::fmt;
use std::iter;
use std::sync::{Arc, Weak};
use std::time::Instant;

use atomic::{AtomicF64, AtomicU64};
use default_registry;
use label::{Label, Labels, LabelsMut};
use metric::{Metric, MetricName, MetricValue};
use timestamp::{self, Timestamp, TimestampMut};
use {Collect, ErrorKind, Registry, Result};

/// `Counter` is a cumulative metric that represents a single numerical value that only ever goes up.
///
/// Cloned counters share the same value.
///
/// # Examples
///
/// ```
/// use prometrics::metrics::CounterBuilder;
///
/// let mut counter = CounterBuilder::new("foo_total").namespace("example").finish().unwrap();
/// assert_eq!(counter.metric_name().to_string(), "example_foo_total");
/// assert_eq!(counter.value(), 0.0);
///
/// counter.increment();
/// assert_eq!(counter.value(), 1.0);
/// ```
#[derive(Debug, Clone)]
pub struct Counter(Arc<Inner>);
impl Counter {
    /// Makes a new `Counter` instance.
    ///
    /// Note that it is recommended to create this via `CounterBuilder`.
    pub fn new(name: &str) -> Result<Self> {
        CounterBuilder::new(name).finish()
    }

    /// Returns the name of this counter.
    pub fn metric_name(&self) -> &MetricName {
        &self.0.name
    }

    /// Returns the help of this counter.
    pub fn help(&self) -> Option<&str> {
        self.0.help.as_ref().map(|h| h.as_ref())
    }

    /// Returns the labels of this counter.
    pub fn labels(&self) -> &Labels {
        &self.0.labels
    }

    /// Returns the mutable labels of this counter.
    pub fn labels_mut(&mut self) -> LabelsMut {
        LabelsMut::new(&self.0.labels, None)
    }

    /// Returns the timestamp of this counter.
    pub fn timestamp(&self) -> &Timestamp {
        &self.0.timestamp
    }

    /// Returns the mutable timestamp of this counter.
    pub fn timestamp_mut(&self) -> TimestampMut {
        TimestampMut::new(&self.0.timestamp)
    }

    /// Returns the value of this counter.
    #[inline]
    pub fn value(&self) -> f64 {
        self.0.value.get()
    }

    /// Increments this counter.
    #[inline]
    pub fn increment(&self) {
        self.0.value.increment()
    }

    /// Adds `count` to this counter.
    #[inline]
    pub fn add(&self, count: f64) -> Result<()> {
        track_assert!(count >= 0.0, ErrorKind::InvalidInput, "count={}", count);
        self.0.value.add(count);
        Ok(())
    }

    /// Adds `count` to this counter.
    #[inline]
    pub fn add_u64(&self, count: u64) {
        self.0.value.add_u64(count);
    }

    /// Measures the exeuction time of `f` and adds its duration to the counter in seconds.
    #[inline]
    pub fn time<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let elapsed = timestamp::duration_to_seconds(start.elapsed());
        self.add(elapsed).expect("Never fails");
        result
    }

    /// Returns a collector for this counter.
    pub fn collector(&self) -> CounterCollector {
        CounterCollector(Arc::downgrade(&self.0))
    }
}
impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.metric_name())?;
        if !self.labels().is_empty() {
            write!(f, "{}", self.labels())?;
        }
        write!(f, " {}", MetricValue(self.value()))?;
        if let Some(timestamp) = self.timestamp().get() {
            write!(f, " {}", timestamp)?;
        }
        Ok(())
    }
}

/// `Counter` builder.
#[derive(Debug)]
pub struct CounterBuilder {
    namespace: Option<String>,
    subsystem: Option<String>,
    name: String,
    help: Option<String>,
    labels: Vec<(String, String)>,
    registries: Vec<Registry>,
}
impl CounterBuilder {
    /// Makes a builder for counters named `name`.
    pub fn new(name: &str) -> Self {
        CounterBuilder {
            namespace: None,
            subsystem: None,
            name: name.to_string(),
            help: None,
            labels: Vec::new(),
            registries: Vec::new(),
        }
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
    pub fn label(&mut self, name: &str, value: &str) -> &mut Self {
        self.labels.retain(|l| l.0 != name);
        self.labels.push((name.to_string(), value.to_string()));
        self.labels.sort();
        self
    }

    /// Adds a registry to which the resulting counters will be registered.
    pub fn registry(&mut self, registry: Registry) -> &mut Self {
        self.registries.push(registry);
        self
    }

    /// Adds the default registry.
    pub fn default_registry(&mut self) -> &mut Self {
        self.registry(default_registry())
    }

    /// Builds a counter.
    ///
    /// # Errors
    ///
    /// This method will return `Err(_)` if any of the name of the metric or labels is malformed.
    pub fn finish(&self) -> Result<Counter> {
        let name = track!(MetricName::new(
            self.namespace.as_ref().map(AsRef::as_ref),
            self.subsystem.as_ref().map(AsRef::as_ref),
            &self.name,
        ))?;
        let labels = track!(self
            .labels
            .iter()
            .map(|&(ref name, ref value)| track!(Label::new(name, value)))
            .collect::<Result<_>>())?;
        let inner = Inner {
            name,
            labels: Labels::new(labels),
            help: self.help.clone(),
            timestamp: Timestamp::new(),
            value: Value::new(),
        };
        let counter = Counter(Arc::new(inner));
        for r in &self.registries {
            r.register(counter.collector());
        }
        Ok(counter)
    }
}

/// `Collect` trait implmentation for `Counter`.
#[derive(Debug)]
pub struct CounterCollector(Weak<Inner>);
impl Collect for CounterCollector {
    type Metrics = iter::Once<Metric>;
    fn collect(&mut self) -> Option<Self::Metrics> {
        self.0
            .upgrade()
            .map(|inner| iter::once(Metric::Counter(Counter(inner))))
    }
}

#[derive(Debug)]
struct Inner {
    name: MetricName,
    labels: Labels,
    help: Option<String>,
    timestamp: Timestamp,
    value: Value,
}

#[derive(Debug)]
struct Value {
    f64: AtomicF64,
    u64: AtomicU64,
}
impl Value {
    fn new() -> Self {
        Value {
            f64: AtomicF64::new(0.0),
            u64: AtomicU64::new(0),
        }
    }

    #[inline]
    fn get(&self) -> f64 {
        self.f64.get() + self.u64.get() as f64
    }

    #[inline]
    fn increment(&self) {
        self.u64.inc();
    }

    #[inline]
    fn add(&self, count: f64) {
        let floor = count.floor() as u64;
        let ceil = count.ceil() as u64;
        if floor == ceil {
            self.u64.add(floor);
        } else {
            self.f64.add(count);
        }
    }

    #[inline]
    fn add_u64(&self, count: u64) {
        self.u64.add(count);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        let mut counter = track_try_unwrap!(CounterBuilder::new("foo_total")
            .namespace("test")
            .subsystem("counter")
            .finish());
        assert_eq!(counter.metric_name().to_string(), "test_counter_foo_total");
        assert_eq!(counter.value(), 0.0);

        counter.increment();
        assert_eq!(counter.value(), 1.0);

        counter.add(3.45).unwrap();
        assert_eq!(counter.value(), 4.45);

        counter.add(2.0).unwrap();
        assert_eq!(counter.value(), 6.45);

        counter.add_u64(2);
        assert_eq!(counter.value(), 8.45);

        assert_eq!(counter.to_string(), "test_counter_foo_total 8.45");
        counter.labels_mut().insert("bar", "baz").unwrap();
        assert_eq!(
            counter.to_string(),
            r#"test_counter_foo_total{bar="baz"} 8.45"#
        );
    }
}
