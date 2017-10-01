use std::fmt;
use std::iter;
use std::sync::{Arc, Weak};

use {Result, Metric, Collector, CollectorRegistry};
use default_registry;
use atomic::AtomicF64;
use label::{Label, Labels, LabelsMut};
use metric::{MetricName, Help};
use timestamp::{Timestamp, TimestampMut};

#[derive(Debug, Clone)]
pub struct Counter(Arc<Inner>);
impl Counter {
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
    pub fn timestamp(&self) -> &Timestamp {
        &self.0.timestamp
    }
    pub fn timestamp_mut(&mut self) -> TimestampMut {
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
    pub fn collector(&self) -> CounterCollector {
        CounterCollector(Arc::downgrade(&self.0))
    }
}
impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())?;
        if !self.labels().is_empty() {
            write!(f, "{}", self.labels())?;
        }
        write!(f, " {}", self.value())?;
        if let Some(timestamp) = self.timestamp().get() {
            write!(f, " {}", timestamp)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct CounterBuilder {
    namespace: Option<String>,
    subsystem: Option<String>,
    name: String,
    help: Option<String>,
    labels: Vec<(String, String)>,
    registries: Vec<CollectorRegistry>,
}
impl CounterBuilder {
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
    pub fn finish(&self) -> Result<Counter> {
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
        let counter = Counter(Arc::new(inner));
        for r in self.registries.iter() {
            track!(r.register(counter.collector()))?;
        }
        Ok(counter)
    }
}

#[derive(Debug, Clone)]
pub struct CounterCollector(Weak<Inner>);
impl Collector for CounterCollector {
    fn collect(&mut self) -> Option<Box<Iterator<Item = Metric>>> {
        self.0.upgrade().map(|inner| {
            let iter: Box<Iterator<Item = _>> =
                Box::new(iter::once(Metric::Counter(Counter(inner))));
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        let mut counter = track_try_unwrap!(
            CounterBuilder::new("foo")
                .namespace("test")
                .subsystem("counter")
                .finish()
        );
        assert_eq!(counter.name(), "test_counter_foo_total");
        assert_eq!(counter.value(), 0.0);

        counter.inc();
        assert_eq!(counter.value(), 1.0);

        counter.inc_by(3.45);
        assert_eq!(counter.value(), 4.45);

        assert_eq!(counter.to_string(), "test_counter_foo_total 4.45");
        track_try_unwrap!(counter.labels_mut().insert("bar", "baz").map(|_| ()));
        assert_eq!(
            counter.to_string(),
            r#"test_counter_foo_total{bar="baz"} 4.45"#
        );
    }
}
