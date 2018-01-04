use std::fmt;
use std::iter;
use std::sync::{Arc, Weak};
use std::time::Instant;

use {Collect, Registry, Result};
use default_registry;
use atomic::AtomicF64;
use label::{Label, Labels, LabelsMut};
use metric::{Metric, MetricName, MetricValue};
use timestamp::{self, Timestamp, TimestampMut};

/// `Gauge` is a metric that represents a single numerical value that can arbitrarily go up and down.
///
/// Cloned gauges share the same value.
#[derive(Debug, Clone)]
pub struct Gauge(Arc<Inner>);
impl Gauge {
    /// Makes a new `Gauge` instance.
    ///
    /// Note that it is recommended to create this via `GaugeBuilder`.
    pub fn new(name: &str) -> Result<Self> {
        GaugeBuilder::new(name).finish()
    }

    /// Returns the name of this gauge.
    pub fn metric_name(&self) -> &MetricName {
        &self.0.name
    }

    /// Returns the help of this gauge.
    pub fn help(&self) -> Option<&str> {
        self.0.help.as_ref().map(|h| h.as_ref())
    }

    /// Returns the labels of this gauge.
    pub fn labels(&self) -> &Labels {
        &self.0.labels
    }

    /// Returns the mutable labels of this gauge.
    pub fn labels_mut(&mut self) -> LabelsMut {
        LabelsMut::new(&self.0.labels, None)
    }

    /// Returns the timestamp of this gauge.
    pub fn timestamp(&self) -> &Timestamp {
        &self.0.timestamp
    }

    /// Returns the mutable timestamp of this gauge.
    pub fn timestamp_mut(&mut self) -> TimestampMut {
        TimestampMut::new(&self.0.timestamp)
    }

    /// Returns the value of this gauge.
    pub fn value(&self) -> f64 {
        self.0.value.get()
    }

    /// Increments this gauge.
    pub fn increment(&self) {
        self.add(1.0);
    }

    /// Adds `count` to this gauge.
    pub fn add(&self, count: f64) {
        self.0.value.update(|v| v + count);
    }

    /// Decrements this gauge.
    pub fn decrement(&self) {
        self.add(-1.0);
    }

    /// Subtracts `count` from this gauge.
    pub fn subtract(&self, count: f64) {
        self.add(-count);
    }

    /// Sets this gauge to `value`.
    pub fn set(&self, value: f64) {
        self.0.value.set(value);
    }

    /// Sets this gauge to the current unixtime in seconds.
    pub fn set_to_current_time(&self) {
        self.set(timestamp::now_unixtime_seconds());
    }

    /// Tracks in-progress processings in some piece of code/function.
    ///
    /// # Examples
    ///
    /// ```
    /// use prometrics::metrics::GaugeBuilder;
    ///
    /// let mut gauge0 = GaugeBuilder::new("foo").finish().unwrap();
    /// let gauge1 = gauge0.clone();
    ///
    /// assert_eq!(gauge0.value(), 0.0);
    /// gauge0.track_inprogress(|| {
    ///     assert_eq!(gauge1.value(), 1.0);
    /// });
    /// assert_eq!(gauge0.value(), 0.0);
    /// ```
    pub fn track_inprogress<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.increment();
        let result = f();
        self.decrement();
        result
    }

    /// Measures the exeuction time of `f` and sets this gauge to its duration in seconds.
    pub fn time<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let elapsed = timestamp::duration_to_unixtime_seconds(start.elapsed());
        self.set(elapsed);
        result
    }

    /// Returns a collector for this gauge.
    pub fn collector(&self) -> GaugeCollector {
        GaugeCollector(Arc::downgrade(&self.0))
    }
}
impl fmt::Display for Gauge {
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

/// `Gauge` builder.
#[derive(Debug)]
pub struct GaugeBuilder {
    namespace: Option<String>,
    subsystem: Option<String>,
    name: String,
    help: Option<String>,
    labels: Vec<(String, String)>,
    initial_value: f64,
    registries: Vec<Registry>,
}
impl GaugeBuilder {
    /// Makes a builder for gauges named `name`.
    pub fn new(name: &str) -> Self {
        GaugeBuilder {
            namespace: None,
            subsystem: None,
            name: name.to_string(),
            help: None,
            labels: Vec::new(),
            initial_value: 0.0,
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

    /// Adds a registry to which the resulting gauges will be registered..
    pub fn registry(&mut self, registry: Registry) -> &mut Self {
        self.registries.push(registry);
        self
    }

    /// Adds the default registry.
    pub fn default_registry(&mut self) -> &mut Self {
        self.registry(default_registry())
    }

    /// Sets the initial value of resulting gauges.
    pub fn initial_value(&mut self, value: f64) -> &mut Self {
        self.initial_value = value;
        self
    }

    /// Builds a gauge.
    ///
    /// # Errors
    ///
    /// This method will return `Err(_)` if any of the name of the metric or labels is malformed.
    pub fn finish(&self) -> Result<Gauge> {
        let name = track!(MetricName::new(
            self.namespace.as_ref().map(AsRef::as_ref),
            self.subsystem.as_ref().map(AsRef::as_ref),
            &self.name,
        ))?;
        let labels = track!(
            self.labels
                .iter()
                .map(|&(ref name, ref value)| track!(Label::new(name, value)))
                .collect::<Result<_>>()
        )?;
        let inner = Inner {
            name,
            labels: Labels::new(labels),
            help: self.help.clone(),
            timestamp: Timestamp::new(),
            value: AtomicF64::new(self.initial_value),
        };
        let gauge = Gauge(Arc::new(inner));
        for r in &self.registries {
            r.register(gauge.collector());
        }
        Ok(gauge)
    }
}

/// `Collect` trait implmentation for `Gauge`.
#[derive(Debug, Clone)]
pub struct GaugeCollector(Weak<Inner>);
impl Collect for GaugeCollector {
    type Metrics = iter::Once<Metric>;
    fn collect(&mut self) -> Option<Self::Metrics> {
        self.0
            .upgrade()
            .map(|inner| iter::once(Metric::Gauge(Gauge(inner))))
    }
}

#[derive(Debug)]
struct Inner {
    name: MetricName,
    labels: Labels,
    help: Option<String>,
    timestamp: Timestamp,
    value: AtomicF64,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        let mut gauge = track_try_unwrap!(GaugeBuilder::new("foo").namespace("test").finish());
        assert_eq!(gauge.metric_name().to_string(), "test_foo");
        assert_eq!(gauge.value(), 0.0);

        gauge.set(2.34);
        assert_eq!(gauge.value(), 2.34);

        assert_eq!(gauge.to_string(), "test_foo 2.34");
        gauge.labels_mut().insert("bar", "baz").unwrap();
        assert_eq!(gauge.to_string(), r#"test_foo{bar="baz"} 2.34"#);
    }
}
