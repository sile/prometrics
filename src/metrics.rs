use std::sync::{Arc, Weak};
use atomic_immut::AtomicImmut;

use Result;
use types::{AtomicF64, Timestamp, Label, Labels};

// #[derive(Debug)]
// pub struct CounterBuilder {
//     namespace: Option<String>,
//     subsystem: Option<String>,
//     name: String,
//     help: Option<String>,
//     labels: Vec<Label>,
//     registries: Vec<CollectorRegistry>,
// }
// impl CounterBuilder {
//     pub fn new(name: &str) -> Self {
//         CounterBuilder {
//             namespace: None,
//             subsystem: None,
//             name: name.to_string(),
//             help: None,
//             labels: Vec::new(),
//             registries: Vec::new(),
//         }
//     }
//     pub fn namespace(&mut self, namespace: &str) -> &mut Self {
//         self.namespace = Some(namespace.to_string());
//         self
//     }
//     pub fn subsystem(&mut self, subsystem: &str) -> &mut Self {
//         self.subsystem = Some(subsystem.to_string());
//         self
//     }
//     pub fn help(&mut self, help: &str) -> &mut Self {
//         self.help = Some(help.to_string());
//         self
//     }
//     pub fn label(&mut self, name: &str, value: &str) -> &mut Self {
//         self.labels.retain(|l| l.name != name);
//         self.labels.push(Label {
//             name: name.to_string(),
//             value: value.to_string(),
//         });
//         self
//     }
//     pub fn register(&mut self, registry: CollectorRegistry) -> &mut Self {
//         self.registries.push(registry);
//         self
//     }
//     pub fn register_default(&mut self) -> &mut Self {
//         self.register(default_registry())
//     }
//     pub fn finish(&self) -> Result<Counter> {
//         panic!()
//     }
// }

#[derive(Debug, Clone)]
pub struct WeakCounter(Weak<CounterInner>);
impl WeakCounter {
    pub fn upgrade(&self) -> Option<Counter> {
        self.0.upgrade().map(Counter)
    }
}

#[derive(Debug, Clone)]
pub struct Counter(Arc<CounterInner>);
impl Counter {
    pub fn name(&self) -> &str {
        &self.0.name
    }
    pub fn help(&self) -> &str {
        &self.0.help
    }
    pub fn labels(&self) -> Labels {
        Labels {
            static_labels: &self.0.static_labels,
            dynamic_labels: self.0.dynamic_labels.load(),
        }
    }
    pub fn timestamp(&self) -> &Timestamp {
        &self.0.timestamp
    }
    // TODO: timestamp_mut, dynamic_labels_mut
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
    pub fn reset(&mut self) {
        self.0.value.set(0.0);
    }
    pub fn weak_clone(&self) -> WeakCounter {
        WeakCounter(Arc::downgrade(&self.0))
    }
}

#[derive(Debug)]
struct CounterInner {
    name: String,
    help: String,
    static_labels: Vec<Label>, // TODO: delete
    dynamic_labels: AtomicImmut<Vec<Label>>,
    timestamp: Timestamp,
    value: AtomicF64,
}

// #[derive(Debug, Clone)]
// pub struct Counter {
//     name: String,
//     description: String,
//     help: String,
//     static_labels: Vec<Label>,
//     dynamic_labels: Arc<AtomicImmut<Vec<Label>>>,
//     timestamp: Arc<Timestamp>,
//     value: Arc<AtomicF64>,
// }
// impl Counter {
//     pub fn name(&self) -> &str {
//         &self.name
//     }
//     pub fn description(&self) -> &str {
//         &self.description
//     }
//     pub fn help(&self) -> &str {
//         &self.help
//     }
//     pub fn labels(&self) -> Labels {
//         Labels {
//             static_labels: &self.static_labels,
//             dynamic_labels: self.dynamic_labels.load(),
//         }
//     }
//     pub fn timestamp(&self) -> &Timestamp {
//         &self.timestamp
//     }
//     pub fn value(&self) -> f64 {
//         self.value.get()
//     }
// }

#[derive(Debug, Clone)]
pub struct Gauge {}

#[derive(Debug, Clone)]
pub struct Summary {}

#[derive(Debug, Clone)]
pub struct Untyped {}

#[derive(Debug, Clone)]
pub struct Histogram {}
