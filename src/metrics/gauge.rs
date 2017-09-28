use std::iter;
use std::sync::{Arc, Weak};
use std::time::Instant;

use {Result, Metric, Collector, CollectorRegistry};
use default_registry;
use atomic::AtomicF64;
use label::{Label, Labels, LabelsMut};
use metric::{MetricName, Help};
use timestamp::{self, Timestamp, TimestampMut};

#[derive(Debug, Clone)]
pub struct Gauge(Arc<Inner>);
impl Gauge {
    pub fn name(&self) -> &str {
        self.0.name.as_str()
    }
    pub fn help(&self) -> Option<&str> {
        self.0.help.as_ref().map(|h| h.0.as_ref())
    }
    pub fn labels(&self) -> &Labels {
        &self.0.labels
    }
    pub fn labels_mut(&mut self) -> LabelsMut {
        LabelsMut::new(&self.0.labels)
    }
    pub fn timetamp(&self) -> &Timestamp {
        &self.0.timestamp
    }
    pub fn timetamp_mut(&mut self) -> TimestampMut {
        TimestampMut(&self.0.timestamp)
    }
    // TODO: get (?)
    pub fn value(&self) -> f64 {
        self.0.value.get()
    }
    pub fn inc(&mut self) {
        self.inc_by(1.0);
    }
    pub fn inc_by(&mut self, count: f64) {
        self.0.value.update(|v| v + count);
    }
    pub fn dec(&mut self) {
        self.inc_by(-1.0);
    }
    pub fn dec_by(&mut self, count: f64) {
        self.inc_by(-count);
    }
    pub fn set(&mut self, value: f64) {
        self.0.value.set(value);
    }
    pub fn set_to_current_time(&mut self) {
        self.set(timestamp::now_unixtime_seconds());
    }
    pub fn track_inprogress<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.inc();
        let result = f();
        self.dec();
        result
    }
    pub fn time<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let elapsed = timestamp::duration_to_unixtime_seconds(start.elapsed());
        self.set(elapsed);
        result
    }
    pub fn collector(&self) -> GaugeCollector {
        GaugeCollector(Arc::downgrade(&self.0))
    }
}

#[derive(Debug)]
pub struct GaugeBuilder {
    namespace: Option<String>,
    subsystem: Option<String>,
    name: String,
    help: Option<String>,
    labels: Vec<(String, String)>,
    value: f64,
    registries: Vec<CollectorRegistry>,
}
impl GaugeBuilder {
    pub fn new(name: &str) -> Self {
        Self::with_value(name, 0.0)
    }
    pub fn with_value(name: &str, initial_value: f64) -> Self {
        GaugeBuilder {
            namespace: None,
            subsystem: None,
            name: name.to_string(),
            help: None,
            labels: Vec::new(),
            value: initial_value,
            registries: Vec::new(),
        }
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
    pub fn finish(&self) -> Result<Gauge> {
        let name = track!(MetricName::new(
            self.namespace.as_ref().map(AsRef::as_ref),
            self.subsystem.as_ref().map(AsRef::as_ref),
            &self.name,
            None,
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
            help: self.help.clone().map(Help),
            timestamp: Timestamp::new(),
            value: AtomicF64::new(self.value),
        };
        let gauge = Gauge(Arc::new(inner));
        for r in self.registries.iter() {
            r.register(gauge.collector());
        }
        Ok(gauge)
    }
}

#[derive(Debug, Clone)]
pub struct GaugeCollector(Weak<Inner>);
impl Collector for GaugeCollector {
    fn collect(&mut self) -> Option<Box<Iterator<Item = Metric>>> {
        self.0.upgrade().map(|inner| {
            let iter: Box<Iterator<Item = _>> = Box::new(iter::once(Metric::Gauge(Gauge(inner))));
            iter
        })
    }
}

#[derive(Debug)]
struct Inner {
    name: MetricName,
    labels: Labels,
    help: Option<Help>,
    timestamp: Timestamp,
    value: AtomicF64,
}
