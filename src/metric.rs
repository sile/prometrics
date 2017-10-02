use std::fmt;
use std::iter;

use {Result, ErrorKind};
use metrics::{Counter, Gauge, Summary, Histogram};

// TODO: MetricsFamily (?)
#[derive(Debug, Clone)]
pub struct MetricFamily {
    name: MetricName,
    help: Option<Help>,

    // TODO
    pub metrics: Metrics,
}
impl MetricFamily {
    pub fn new(metric: Metric) -> Self {
        match metric {
            Metric::Counter(m) => {
                MetricFamily {
                    name: MetricName(m.name().to_string()),
                    help: m.help().map(|h| Help(h.to_string())),
                    metrics: Metrics::Counter(vec![m]),
                }
            }
            Metric::Gauge(m) => {
                MetricFamily {
                    name: MetricName(m.name().to_string()),
                    help: m.help().map(|h| Help(h.to_string())),
                    metrics: Metrics::Gauge(vec![m]),
                }
            }
            Metric::Summary(m) => {
                MetricFamily {
                    name: MetricName(m.quantile_name().to_string()),
                    help: m.help().map(|h| Help(h.to_string())),
                    metrics: Metrics::Summary(vec![m]),
                }
            }
            Metric::Histogram(m) => {
                MetricFamily {
                    name: MetricName(m.bucket_name().to_string()),
                    help: m.help().map(|h| Help(h.to_string())),
                    metrics: Metrics::Histogram(vec![m]),
                }
            }
        }
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn help(&self) -> Option<&str> {
        self.help.as_ref().map(|h| h.0.as_ref())
    }
    pub fn kind(&self) -> MetricKind {
        match self.metrics {
            Metrics::Counter(_) => MetricKind::Counter,
            Metrics::Gauge(_) => MetricKind::Gauge,
            Metrics::Summary(_) => MetricKind::Summary,
            Metrics::Histogram(_) => MetricKind::Histogram,
        }
    }
    pub(crate) fn same_family(&self, metric: &Metric) -> bool {
        (self.name(), self.kind()) == (metric.name(), metric.kind())
    }
    pub(crate) fn push(&mut self, metric: Metric) {
        match metric {
            Metric::Counter(m) => {
                if let Metrics::Counter(ref mut v) = self.metrics {
                    v.push(m);
                }
            }
            Metric::Gauge(m) => {
                if let Metrics::Gauge(ref mut v) = self.metrics {
                    v.push(m);
                }
            }
            Metric::Summary(m) => {
                if let Metrics::Summary(ref mut v) = self.metrics {
                    v.push(m);
                }
            }
            Metric::Histogram(m) => {
                if let Metrics::Histogram(ref mut v) = self.metrics {
                    v.push(m);
                }
            }
        }
    }
}
impl fmt::Display for MetricFamily {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(help) = self.help() {
            // TODO: escape
            writeln!(f, "# HELP {} {}", self.name(), help)?;
        }
        writeln!(f, "# TYPE {} {}", self.name(), self.kind())?;
        writeln!(f, "{}", self.metrics)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Metrics {
    Counter(Vec<Counter>),
    Gauge(Vec<Gauge>),
    Summary(Vec<Summary>),
    Histogram(Vec<Histogram>),
}
impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Metrics::Counter(ref v) => {
                for m in v.iter() {
                    writeln!(f, "{}", m)?;
                }
            }
            Metrics::Gauge(ref v) => {
                for m in v.iter() {
                    writeln!(f, "{}", m)?;
                }
            }
            Metrics::Summary(ref v) => {
                for m in v.iter() {
                    writeln!(f, "{}", m)?;
                }
            }
            Metrics::Histogram(ref v) => {
                for m in v.iter() {
                    writeln!(f, "{}", m)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Metric {
    Counter(Counter),
    Gauge(Gauge),
    Summary(Summary),
    Histogram(Histogram),
}
impl Metric {
    pub fn name(&self) -> &str {
        match *self {
            Metric::Counter(ref m) => m.name(),
            Metric::Gauge(ref m) => m.name(),
            Metric::Summary(ref m) => m.quantile_name(),
            Metric::Histogram(ref m) => m.bucket_name(),
        }
    }
    pub fn kind(&self) -> MetricKind {
        match *self {
            Metric::Counter(_) => MetricKind::Counter,
            Metric::Gauge(_) => MetricKind::Gauge,
            Metric::Summary(_) => MetricKind::Summary,
            Metric::Histogram(_) => MetricKind::Histogram,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetricKind {
    Counter,
    Gauge,
    Summary,
    Histogram,
}
impl fmt::Display for MetricKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MetricKind::Counter => write!(f, "counter"),
            MetricKind::Gauge => write!(f, "gauge"),
            MetricKind::Summary => write!(f, "summary"),
            MetricKind::Histogram => write!(f, "histogram"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricName(String);
impl MetricName {
    pub fn new(
        namespace: Option<&str>,
        subsystem: Option<&str>,
        name: &str,
        suffix: Option<&str>,
    ) -> Result<Self> {
        let fullname = namespace
            .into_iter()
            .chain(subsystem.into_iter())
            .chain(iter::once(name))
            .chain(suffix.into_iter())
            .collect::<Vec<_>>()
            .join("_");
        track!(validate_metric_name(&fullname), "name={:?}", fullname)?;
        Ok(MetricName(fullname))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// TODO: escape newline and '\\'
#[derive(Debug, Clone)]
pub struct Help(pub String);

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

// // TODO: primitive(?)
// use std::sync::{Arc, Weak};
// use std::sync::atomic::{AtomicUsize, AtomicIsize, Ordering};
// use std::time::{Duration, SystemTime};
// use atomic_immut::AtomicImmut;

// use {Result, ErrorKind};

// #[derive(Debug)]
// pub struct MetricBuilder {
//     name: String,
//     description: String,
//     help: String,
//     labels: Labels,
// }
// impl MetricBuilder {
//     pub fn new(name: &str) -> Self {
//         MetricBuilder {
//             name: name.to_string(),
//             description: String::new(),
//             help: String::new(),
//             labels: Labels(Vec::new()),
//         }
//     }
//     pub fn description(&mut self, description: &str) -> &mut Self {
//         self.description = description.to_string();
//         self
//     }
//     pub fn help(&mut self, help: &str) -> &mut Self {
//         self.help = help.to_string();
//         self
//     }
//     pub fn label(&mut self, name: &str, value: &str) -> &mut Self {
//         self.labels.0.retain(|l| l.name != name);
//         self.labels.0.push(Label {
//             name: name.to_string(),
//             value: value.to_string(),
//         });
//         self
//     }
//     pub fn finish(&self) -> Result<(Metric, Value)> {
//         track!(validate_metric_name(&self.name), "name={:?}", self.name)?;
//         for label in self.labels.0.iter() {
//             track!(validate_label_name(&label.name), "name={:?}", label.name)?;
//         }

//         let value = Value(Arc::new(AtomicF64::new()));
//         let mut metric = Metric {
//             name: self.name.clone(),
//             description: self.description.clone(),
//             help: self.help.clone(),
//             labels: self.labels.clone(),
//             value: Arc::downgrade(&value.0),
//         };
//         metric.labels.0.sort();
//         Ok((metric, value))
//     }
// }

// fn validate_metric_name(name: &str) -> Result<()> {
//     // REGEX: [a-zA-Z_:][a-zA-Z0-9_:]*
//     track_assert!(!name.is_empty(), ErrorKind::InvalidInput);
//     match name.as_bytes()[0] as char {
//         'a'...'z' | 'A'...'Z' | '_' | ':' => {}
//         _ => track_panic!(ErrorKind::InvalidInput),
//     }
//     for c in name.chars().skip(1) {
//         match c {
//             'a'...'z' | 'A'...'Z' | '0'...'9' | '_' | ':' => {}
//             _ => track_panic!(ErrorKind::InvalidInput),
//         }
//     }
//     Ok(())
// }

// fn validate_label_name(name: &str) -> Result<()> {
//     // REGEX: [a-zA-Z_][a-zA-Z0-9_]*
//     track_assert!(!name.is_empty(), ErrorKind::InvalidInput);
//     track_assert!(!name.starts_with("__"), ErrorKind::InvalidInput, "Reserved");
//     match name.as_bytes()[0] as char {
//         'a'...'z' | 'A'...'Z' | '_' => {}
//         _ => track_panic!(ErrorKind::InvalidInput),
//     }
//     for c in name.chars().skip(1) {
//         match c {
//             'a'...'z' | 'A'...'Z' | '0'...'9' | '_' => {}
//             _ => track_panic!(ErrorKind::InvalidInput),
//         }
//     }
//     Ok(())
// }

// pub enum Metric2 {
//     Counter(CounterMetric),
//     Gauge(GaugeMetric),
//     Summary(SummaryMetric),
//     Untyped(UntypedMetric),
//     Histogram(HistogramMetric),
// }

// struct DynamicLabels(AtomicImmut<Vec<Label>>);

// struct Common {
//     name: String,
//     help: String,
//     static_labels: Labels,
//     dynamic_labels: Arc<DynamicLabels>,
//     timestamp: Arc<Timestamp>,
// }

// pub struct CounterMetric {
//     common: Common,
//     value: Arc<AtomicF64>,
// }

// pub struct GaugeMetric {
//     common: Common,
//     value: Arc<AtomicF64>,
// }

// pub struct SummaryMetric {
//     common: Common,
//     sample_count: Arc<AtomicUsize>, // TODO: u64
//     sample_sum: Arc<AtomicF64>,

//     window: Duration,
//     samples: Arc<Vec<Sample>>, // TODO: Receiver
//     quantiles: Vec<f64>,
// }

// pub struct Sample {
//     time: SystemTime,
//     value: f64,
// }

// pub struct UntypedMetric {
//     common: Common,
//     value: Arc<AtomicF64>,
// }

// pub struct HistogramMetric {
//     common: Common,
//     sample_count: Arc<AtomicUsize>, // TODO: u64
//     sample_sum: Arc<AtomicF64>,
//     buckets: Arc<Vec<Bucket>>,
// }

// pub struct Bucket {
//     count: u64, // TODO: convert to cumulative in encoding phase
//     upper_bound: f64, // inclusive
// }

// // TODO: enum
// #[derive(Debug, Clone)]
// pub struct Metric {
//     name: String,
//     description: String,
//     help: String,
//     labels: Labels,
//     value: Weak<AtomicF64>, // TODO: Arc
// }
// impl Metric {
//     pub fn name(&self) -> &str {
//         &self.name
//     }
//     pub fn description(&self) -> &str {
//         &self.description
//     }
//     pub fn help(&self) -> &str {
//         &self.help
//     }
//     pub fn labels(&self) -> &Labels {
//         &self.labels
//     }
//     pub fn value(&self) -> Option<f64> {
//         self.value.upgrade().map(|v| v.get())
//     }
// }

// #[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
// pub struct Labels(Vec<Label>);

// #[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
// pub struct Label {
//     name: String,
//     value: String,
// }
// impl Label {
//     pub fn name(&self) -> &str {
//         &self.name
//     }
//     pub fn value(&self) -> &str {
//         &self.value
//     }
// }

// #[derive(Debug, Clone)]
// pub struct Value(Arc<AtomicF64>);

// // TODO: cfg
// #[derive(Debug)]
// struct AtomicF64(AtomicUsize);
// impl AtomicF64 {
//     pub fn new() -> Self {
//         AtomicF64(AtomicUsize::new(0.0 as usize))
//     }
//     pub fn get(&self) -> f64 {
//         self.0.load(Ordering::SeqCst) as f64
//     }
// }

// // TODO: cfg
// #[derive(Debug)]
// pub struct Timestamp(AtomicIsize);
// impl Timestamp {
//     pub fn none() -> Self {
//         Timestamp(AtomicIsize::new(::std::isize::MIN))
//     }
// }
