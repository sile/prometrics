use std::fmt;

use {Result, ErrorKind};
use metrics::{Counter, Gauge, Summary, Histogram};

// TODO: MetricsFamily (?)
#[derive(Debug, Clone)]
pub struct MetricFamily {
    name: MetricName,
    help: Option<String>,

    // TODO
    pub metrics: Metrics,
}
impl MetricFamily {
    pub fn new(metric: Metric) -> Self {
        match metric {
            Metric::Counter(m) => {
                MetricFamily {
                    name: m.metric_name().clone(),
                    help: m.help().map(|h| h.to_string()),
                    metrics: Metrics::Counter(vec![m]),
                }
            }
            Metric::Gauge(m) => {
                MetricFamily {
                    name: m.metric_name().clone(),
                    help: m.help().map(|h| h.to_string()),
                    metrics: Metrics::Gauge(vec![m]),
                }
            }
            Metric::Summary(m) => {
                MetricFamily {
                    name: m.metric_name().clone(),
                    help: m.help().map(|h| h.to_string()),
                    metrics: Metrics::Summary(vec![m]),
                }
            }
            Metric::Histogram(m) => {
                MetricFamily {
                    name: m.metric_name().clone(),
                    help: m.help().map(|h| h.to_string()),
                    metrics: Metrics::Histogram(vec![m]),
                }
            }
        }
    }
    pub fn name(&self) -> &MetricName {
        &self.name
    }
    pub fn help(&self) -> Option<&str> {
        self.help.as_ref().map(|h| h.as_ref())
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
    pub fn name(&self) -> &MetricName {
        match *self {
            Metric::Counter(ref m) => m.metric_name(),
            Metric::Gauge(ref m) => m.metric_name(),
            Metric::Summary(ref m) => m.metric_name(),
            Metric::Histogram(ref m) => m.metric_name(),
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MetricName {
    namespace: Option<String>,
    subsystem: Option<String>,
    name: String,
}
impl MetricName {
    pub fn new(namespace: Option<&str>, subsystem: Option<&str>, name: &str) -> Result<Self> {
        if let Some(s) = namespace {
            track!(Self::validate_name(s), "{:?}", s)?;
        }
        if let Some(s) = subsystem {
            track!(Self::validate_name(s), "{:?}", s)?;
        }
        track!(Self::validate_name(name), "{:?}", name)?;

        Ok(MetricName {
            namespace: namespace.map(|s| s.to_owned()),
            subsystem: subsystem.map(|s| s.to_owned()),
            name: name.to_string(),
        })
    }
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_ref().map(|s| s.as_ref())
    }
    pub fn subsystem(&self) -> Option<&str> {
        self.subsystem.as_ref().map(|s| s.as_ref())
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    fn validate_name(name: &str) -> Result<()> {
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
}
impl fmt::Display for MetricName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref s) = self.namespace {
            write!(f, "{}_", s)?;
        }
        if let Some(ref s) = self.subsystem {
            write!(f, "{}_", s)?;
        }
        write!(f, "{}", self.name)?;
        Ok(())
    }
}
