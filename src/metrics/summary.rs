use std::cmp;
use std::collections::VecDeque;
use std::fmt;
use std::iter;
use std::sync::{Arc, Weak, Mutex};
use std::time::{Instant, Duration, SystemTime};

use {Result, ErrorKind, Collect, Registry};
use default_registry;
use atomic::{AtomicF64, AtomicU64};
use label::{Label, Labels, LabelsMut};
use metric::{Metric, MetricName, MetricValue};
use quantile::Quantile;
use timestamp::{self, Timestamp, TimestampMut};

/// `Summary` samples observations (usually things like request durations and response sizes).
///
/// It provides a total count of observations and a sum of all observed values,
/// and it calculates configurable quantiles over a sliding time window.
///
/// Cloned summaries share the same buckets.
#[derive(Debug, Clone)]
pub struct Summary(Arc<Inner>);
impl Summary {
    /// Makes a new `Summary` instance.
    ///
    /// Note that it is recommended to create this via `SummaryBuilder`.
    pub fn new(name: &str, window: Duration) -> Result<Self> {
        SummaryBuilder::new(name, window).finish()
    }

    /// Returns the name of this summary.
    pub fn metric_name(&self) -> &MetricName {
        &self.0.quantile_name
    }

    /// Returns the help of this summary.
    pub fn help(&self) -> Option<&str> {
        self.0.help.as_ref().map(|h| h.as_ref())
    }

    /// Returns the user defined labels of this summary.
    pub fn labels(&self) -> &Labels {
        &self.0.labels
    }

    /// Returns the mutable user defined labels of this summary.
    pub fn labels_mut(&mut self) -> LabelsMut {
        LabelsMut::new(&self.0.labels, Some("quantile"))
    }

    /// Returns the timestamp of this summary.
    pub fn timetamp(&self) -> &Timestamp {
        &self.0.timestamp
    }

    /// Returns the mutable timestamp of this summary.
    pub fn timetamp_mut(&mut self) -> TimestampMut {
        TimestampMut::new(&self.0.timestamp)
    }

    /// Returns the total observation count.
    pub fn count(&self) -> u64 {
        self.0.count.get()
    }

    /// Returns the sum of the observed values.
    pub fn sum(&self) -> f64 {
        self.0.sum.get()
    }

    /// Calculates and returns the quantile-value pairs of this summary.
    pub fn quantiles(&self) -> Vec<(Quantile, f64)> {
        let mut samples = self.with_current_samples(|_, samples| {
            samples
                .iter()
                .map(|&(_, v)| v)
                .filter(|v| !v.is_nan())
                .collect::<Vec<_>>()
        });
        samples.sort_by(|a, b| a.partial_cmp(b).expect("Never fails"));

        if samples.is_empty() {
            return Vec::new();
        }
        let count = samples.len();
        self.0
            .quantiles
            .iter()
            .map(|&quantile| {
                let index = cmp::min(count, (quantile.as_f64() * count as f64).floor() as usize);
                (quantile, samples[index])
            })
            .collect()
    }

    /// Observes a value.
    pub fn observe(&mut self, value: f64) {
        self.with_current_samples(|now, samples| { samples.push_back((now, value)); });
        self.0.count.inc();
        self.0.sum.update(|v| v + value);
    }

    /// Measures the exeuction time of `f` and observes its duration in seconds.
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

    /// Returns a collector for this histogram.
    pub fn collector(&self) -> SummaryCollector {
        SummaryCollector(Arc::downgrade(&self.0))
    }

    fn with_current_samples<F, T>(&self, f: F) -> T
    where
        F: FnOnce(SystemTime, &mut VecDeque<(SystemTime, f64)>) -> T,
    {
        let now = SystemTime::now();
        if let Ok(mut samples) = self.0.samples.lock() {
            while samples
                .front()
                .and_then(|s| now.duration_since(s.0).ok())
                .and_then(|d| if d > self.0.window { Some(()) } else { None })
                .is_some()
            {
                samples.pop_front();
            }
            f(now, &mut samples)
        } else {
            f(now, &mut VecDeque::new())
        }
    }
}
impl fmt::Display for Summary {
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

        for (quantile, value) in self.quantiles() {
            write!(
                f,
                "{}{{quantile=\"{}\"",
                self.metric_name(),
                quantile.as_f64()
            )?;
            for label in self.labels().iter() {
                write!(f, ",{}={:?}", label.name(), label.value())?;
            }
            writeln!(f, "}} {}{}", MetricValue(value), timestamp)?;
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

/// `Summary` builder.
#[derive(Debug)]
pub struct SummaryBuilder {
    namespace: Option<String>,
    subsystem: Option<String>,
    name: String,
    help: Option<String>,
    labels: Vec<(String, String)>,
    window: Duration,
    quantiles: Vec<f64>,
    registries: Vec<Registry>,
}
impl SummaryBuilder {
    /// Makes a builder for summary named `name`.
    pub fn new(name: &str, window: Duration) -> Self {
        SummaryBuilder {
            namespace: None,
            subsystem: None,
            name: name.to_string(),
            help: None,
            labels: Vec::new(),
            window,
            quantiles: Vec::new(),
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
    ///
    /// The name `"quantile"` is reserved for designating summary quantiles.
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

    /// Adds a quantile.
    pub fn quantile(&mut self, quantile: f64) -> &mut Self {
        self.quantiles.push(quantile);
        self
    }

    /// Builds a summary.
    pub fn finish(&self) -> Result<Summary> {
        let namespace = self.namespace.as_ref().map(AsRef::as_ref);
        let subsystem = self.subsystem.as_ref().map(AsRef::as_ref);
        let quantile_name = track!(MetricName::new(namespace, subsystem, &self.name))?;
        let labels = track!(
            self.labels
                .iter()
                .map(|&(ref name, ref value)| {
                    track_assert_ne!(name, "quantile", ErrorKind::InvalidInput);
                    track!(Label::new(name, value))
                })
                .collect::<Result<_>>()
        )?;
        let mut quantiles = track!(
            self.quantiles
                .iter()
                .map(|quantile| track!(Quantile::new(*quantile)))
                .collect::<Result<Vec<_>>>()
        )?;
        quantiles.sort_by(|a, b| {
            a.as_f64().partial_cmp(&b.as_f64()).expect("Never fails")
        });
        let inner = Inner {
            quantile_name,
            labels: Labels::new(labels),
            help: self.help.clone(),
            timestamp: Timestamp::new(),
            window: self.window,
            quantiles,
            samples: Mutex::new(VecDeque::new()),
            count: AtomicU64::new(0),
            sum: AtomicF64::new(0.0),
        };
        let summary = Summary(Arc::new(inner));
        for r in &self.registries {
            track!(r.register(summary.collector()))?;
        }
        Ok(summary)
    }
}

/// `Collect` trait implmentation for `Summary`.
#[derive(Debug, Clone)]
pub struct SummaryCollector(Weak<Inner>);
impl Collect for SummaryCollector {
    type Metrics = iter::Once<Metric>;
    fn collect(&mut self) -> Option<Self::Metrics> {
        self.0.upgrade().map(|inner| {
            iter::once(Metric::Summary(Summary(inner)))
        })
    }
}

#[derive(Debug)]
struct Inner {
    quantile_name: MetricName,
    labels: Labels,
    help: Option<String>,
    timestamp: Timestamp,
    window: Duration,
    quantiles: Vec<Quantile>,
    samples: Mutex<VecDeque<(SystemTime, f64)>>,
    count: AtomicU64,
    sum: AtomicF64,
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::*;

    #[test]
    fn it_works() {
        let mut summary = track_try_unwrap!(
            SummaryBuilder::new("foo", Duration::from_secs(10))
                .quantile(0.25)
                .quantile(0.5)
                .quantile(0.75)
                .finish()
        );
        assert_eq!(summary.metric_name().to_string(), "foo");

        summary.observe(7.0);
        summary.observe(12.0);
        summary.observe(50.0);
        summary.observe(10.0);
        summary.observe(33.0);
        assert_eq!(
            summary
                .quantiles()
                .into_iter()
                .map(|(q, v)| (q.as_f64(), v))
                .collect::<Vec<_>>(),
            [(0.25, 10.0), (0.50, 12.0), (0.75, 33.0)]
        );
        assert_eq!(summary.count(), 5);
        assert_eq!(summary.sum(), 112.0);

        assert_eq!(
            summary.to_string(),
            r#"foo{quantile="0.25"} 10
foo{quantile="0.5"} 12
foo{quantile="0.75"} 33
foo_sum 112
foo_count 5"#
        );
    }
}
