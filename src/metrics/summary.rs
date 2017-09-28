// TODO:
use std::iter;
use std::sync::{Arc, Weak};

use {Result, Metric, Collector, CollectorRegistry};
use default_registry;
use atomic::AtomicF64;
use label::{Label, Labels, LabelsMut};
use metric::{MetricName, Help};
use timestamp::{Timestamp, TimestampMut};

#[derive(Debug, Clone)]
pub struct Summary(Arc<Inner>);
impl Summary {
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
    pub fn value(&self) -> f64 {
        self.0.value.get()
    }
    pub fn inc(&mut self) {
        self.0.value.update(|v| v + 1.0);
    }
    pub fn inc_by(&mut self, count: f64) {
        assert!(count >= 0.0);
        self.0.value.update(|v| v + count);
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
    registries: Vec<CollectorRegistry>,
}
impl SummaryBuilder {
    pub fn new(name: &str) -> Self {
        SummaryBuilder {
            namespace: None,
            subsystem: None,
            name: name.to_string(),
            help: None,
            labels: Vec::new(),
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
    pub fn finish(&self) -> Result<Summary> {
        let name = track!(MetricName::new(
            self.namespace.as_ref().map(AsRef::as_ref),
            self.subsystem.as_ref().map(AsRef::as_ref),
            &self.name,
            Some("total"),
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
            value: AtomicF64::new(0.0),
        };
        let summary = Summary(Arc::new(inner));
        for r in self.registries.iter() {
            r.register(summary.collector());
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
    name: MetricName,
    labels: Labels,
    help: Option<Help>,
    timestamp: Timestamp,
    value: AtomicF64,
}
