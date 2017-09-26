// TODO: primitive(?)
use std::sync::{Arc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};

use {Result, ErrorKind};

#[derive(Debug)]
pub struct MetricBuilder {
    name: String,
    description: String,
    help: String,
    labels: Labels,
}
impl MetricBuilder {
    pub fn new(name: &str) -> Self {
        MetricBuilder {
            name: name.to_string(),
            description: String::new(),
            help: String::new(),
            labels: Labels(Vec::new()),
        }
    }
    pub fn description(&mut self, description: &str) -> &mut Self {
        self.description = description.to_string();
        self
    }
    pub fn help(&mut self, help: &str) -> &mut Self {
        self.help = help.to_string();
        self
    }
    pub fn label(&mut self, name: &str, value: &str) -> &mut Self {
        self.labels.0.retain(|l| l.name != name);
        self.labels.0.push(Label {
            name: name.to_string(),
            value: value.to_string(),
        });
        self
    }
    pub fn finish(&self) -> Result<(Metric, Value)> {
        track!(validate_metric_name(&self.name), "name={:?}", self.name)?;
        for label in self.labels.0.iter() {
            track!(validate_label_name(&label.name), "name={:?}", label.name)?;
        }

        let value = Value(Arc::new(AtomicF64::new()));
        let mut metric = Metric {
            name: self.name.clone(),
            description: self.description.clone(),
            help: self.help.clone(),
            labels: self.labels.clone(),
            value: Arc::downgrade(&value.0),
        };
        metric.labels.0.sort();
        Ok((metric, value))
    }
}

fn validate_metric_name(name: &str) -> Result<()> {
    // REGEX: [a-zA-Z_:][a-zA-Z0-9_:]*
    track_assert!(!name.is_empty(), ErrorKind::InvalidInput);
    match name.as_bytes()[0] as char {
        'a'...'z' | 'A'...'Z' | '_' | ':' => {}
        _ => track_panic!(ErrorKind::InvalidInput),
    }
    for c in name.chars().skip(1) {
        match c {
            'a'...'z' | 'A'...'Z' | '0'...'9' | '_' | ':' => {}
            _ => track_panic!(ErrorKind::InvalidInput),
        }
    }
    Ok(())
}

fn validate_label_name(name: &str) -> Result<()> {
    // REGEX: [a-zA-Z_][a-zA-Z0-9_]*
    track_assert!(!name.is_empty(), ErrorKind::InvalidInput);
    track_assert!(!name.starts_with("__"), ErrorKind::InvalidInput, "Reserved");
    match name.as_bytes()[0] as char {    
        'a'...'z' | 'A'...'Z' | '_' => {}
        _ => track_panic!(ErrorKind::InvalidInput),
    }
    for c in name.chars().skip(1) {
        match c {
            'a'...'z' | 'A'...'Z' | '0'...'9' | '_' => {}
            _ => track_panic!(ErrorKind::InvalidInput),
        }
    }
    Ok(())
}

// TODO: enum
#[derive(Debug, Clone)]
pub struct Metric {
    name: String,
    description: String,
    help: String,
    labels: Labels,
    value: Weak<AtomicF64>, // TODO: Arc
}
impl Metric {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn help(&self) -> &str {
        &self.help
    }
    pub fn labels(&self) -> &Labels {
        &self.labels
    }
    pub fn value(&self) -> Option<f64> {
        self.value.upgrade().map(|v| v.get())
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Labels(Vec<Label>);

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Label {
    name: String,
    value: String,
}
impl Label {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone)]
pub struct Value(Arc<AtomicF64>);

// TODO: cfg
#[derive(Debug)]
struct AtomicF64(AtomicUsize);
impl AtomicF64 {
    pub fn new() -> Self {
        AtomicF64(AtomicUsize::new(0.0 as usize))
    }
    pub fn get(&self) -> f64 {
        self.0.load(Ordering::SeqCst) as f64
    }
}
