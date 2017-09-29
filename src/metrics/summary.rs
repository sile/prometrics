use std::collections::VecDeque;
use std::iter;
use std::sync::{Arc, Weak, Mutex};
use std::time::{Instant, Duration, SystemTime};

use {Result, Metric, Collector, CollectorRegistry};
use default_registry;
use atomic::{AtomicF64, AtomicU64};
use label::{Label, Labels};
use metric::{MetricName, Help};
use timestamp::{self, Timestamp, TimestampMut};

#[derive(Debug, Clone)]
pub struct Summary(Arc<Inner>);
impl Summary {
    pub fn quantile_name(&self) -> &str {
        self.0.quantile_name.as_str()
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
    // TODO: SummaryLabelsMut
    // pub fn labels_mut(&mut self) -> LabelsMut {
    //     LabelsMut::new(&self.0.labels)
    // }
    pub fn timetamp(&self) -> &Timestamp {
        &self.0.timestamp
    }
    pub fn timetamp_mut(&mut self) -> TimestampMut {
        TimestampMut(&self.0.timestamp)
    }
    pub fn quantiles(&self) -> Vec<(f64, f64)> {
        // TODO: 共通化
        let mut samples = if let Ok(mut samples) = self.0.samples.lock() {
            let now = SystemTime::now();
            while samples
                .front()
                .and_then(|s| now.duration_since(s.0).ok())
                .and_then(|d| if d > self.0.window { Some(()) } else { None })
                .is_some()
            {
                samples.pop_front();
            }
            samples
                .iter()
                .map(|&(_, v)| v)
                .filter(|v| !v.is_nan())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if samples.is_empty() {
            return Vec::new();
        }

        samples.sort_by(|a, b| a.partial_cmp(&b).expect("Never fails"));
        let count = samples.len() as f64;
        self.0
            .quantiles
            .iter()
            .map(|&quantile| {
                // TODO:
                let mut index = (quantile * count).floor() as usize;
                if index == samples.len() {
                    index -= 1;
                }
                (quantile, samples[index])
            })
            .collect()
    }
    pub fn observe(&mut self, value: f64) {
        assert_ne!(value, ::std::f64::NAN);
        if let Ok(mut samples) = self.0.samples.lock() {
            let now = SystemTime::now();
            samples.push_back((now, value));
            while samples
                .front()
                .and_then(|s| now.duration_since(s.0).ok())
                .and_then(|d| if d > self.0.window { Some(()) } else { None })
                .is_some()
            {
                samples.pop_front();
            }
        }
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
    pub fn collector(&self) -> SummaryCollector {
        SummaryCollector(Arc::downgrade(&self.0))
    }
}

#[derive(Debug)]
pub struct SummaryBuilder {
    namespace: Option<String>,
    subsystem: Option<String>,
    name: String,
    help: Option<String>,
    labels: Vec<(String, String)>,
    window: Duration,
    quantiles: Vec<f64>,
    registries: Vec<CollectorRegistry>,
}
impl SummaryBuilder {
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
    pub fn quantile(&mut self, quantile: f64) -> &mut Self {
        assert_ne!(quantile, ::std::f64::NAN);
        assert!(quantile >= 0.0);
        assert!(quantile <= 1.0);
        self.quantiles.push(quantile);
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
        assert_ne!(name, "quantile"); // TODO: validate in `finish` method
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
    pub fn finish(&self) -> Result<Summary> {
        let namespace = self.namespace.as_ref().map(AsRef::as_ref);
        let subsystem = self.subsystem.as_ref().map(AsRef::as_ref);
        let quantile_name = track!(MetricName::new(namespace, subsystem, &self.name, None))?;
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
        let mut quantiles = self.quantiles.clone();
        quantiles.sort_by(|a, b| a.partial_cmp(b).expect("Never fails"));
        let inner = Inner {
            quantile_name,
            count_name,
            sum_name,
            labels: Labels::new(labels),
            help: self.help.clone().map(Help),
            timestamp: Timestamp::new(),
            window: self.window.clone(),
            quantiles,
            samples: Mutex::new(VecDeque::new()),
            count: AtomicU64::new(0),
            sum: AtomicF64::new(0.0),
        };
        let summary = Summary(Arc::new(inner));
        for r in self.registries.iter() {
            track!(r.register(summary.collector()))?;
        }
        Ok(summary)
    }
}

#[derive(Debug, Clone)]
pub struct SummaryCollector(Weak<Inner>);
impl Collector for SummaryCollector {
    fn collect(&mut self) -> Option<Box<Iterator<Item = Metric>>> {
        self.0.upgrade().map(|inner| {
            let iter: Box<Iterator<Item = _>> =
                Box::new(iter::once(Metric::Summary(Summary(inner))));
            iter
        })
    }
}

#[derive(Debug)]
struct Inner {
    quantile_name: MetricName,
    count_name: MetricName,
    sum_name: MetricName,
    labels: Labels,
    help: Option<Help>,
    timestamp: Timestamp,
    window: Duration,
    quantiles: Vec<f64>,
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
        assert_eq!(summary.quantile_name(), "foo");
        assert_eq!(summary.count_name(), "foo_count");
        assert_eq!(summary.sum_name(), "foo_sum");

        summary.observe(7.0);
        summary.observe(12.0);
        summary.observe(50.0);
        summary.observe(10.0);
        summary.observe(33.0);
        assert_eq!(
            summary.quantiles(),
            [(0.25, 10.0), (0.50, 12.0), (0.75, 33.0)]
        );
        assert_eq!(summary.count(), 5);
        assert_eq!(summary.sum(), 112.0);
    }
}
