//! Metric.
use std;
use std::fmt;

use {ErrorKind, Result};
use metrics::{Counter, Gauge, Histogram, Summary};

/// Metric.
///
/// # References
///
/// - [Metric types](https://prometheus.io/docs/concepts/metric_types/)
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Metric {
    Counter(Counter),
    Gauge(Gauge),
    Summary(Summary),
    Histogram(Histogram),
}
impl Metric {
    /// Returns the name of this metric.
    pub fn name(&self) -> &MetricName {
        match *self {
            Metric::Counter(ref m) => m.metric_name(),
            Metric::Gauge(ref m) => m.metric_name(),
            Metric::Summary(ref m) => m.metric_name(),
            Metric::Histogram(ref m) => m.metric_name(),
        }
    }

    /// Returns tye kind of this metric.
    pub fn kind(&self) -> MetricKind {
        match *self {
            Metric::Counter(_) => MetricKind::Counter,
            Metric::Gauge(_) => MetricKind::Gauge,
            Metric::Summary(_) => MetricKind::Summary,
            Metric::Histogram(_) => MetricKind::Histogram,
        }
    }
}
impl From<Counter> for Metric {
    fn from(f: Counter) -> Self {
        Metric::Counter(f)
    }
}
impl From<Gauge> for Metric {
    fn from(f: Gauge) -> Self {
        Metric::Gauge(f)
    }
}
impl From<Histogram> for Metric {
    fn from(f: Histogram) -> Self {
        Metric::Histogram(f)
    }
}
impl From<Summary> for Metric {
    fn from(f: Summary) -> Self {
        Metric::Summary(f)
    }
}

/// Metric name.
///
/// A metric name is a sequence of characters that match the regex `[a-zA-Z_:][a-zA-Z0-9_:]*`.
/// It consists of three parts: `{namespace}_{subsystem}_{name}` of which only the `name` is mandatory.
///
/// # References
///
/// - [Metric name and labels](https://prometheus.io/docs/concepts/data_model/#metric-names-and-labels)
/// - [Metric names](https://prometheus.io/docs/instrumenting/writing_clientlibs/#metric-names)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MetricName {
    namespace: Option<String>,
    subsystem: Option<String>,
    name: String,
}
impl MetricName {
    /// Returns the namespace part of this.
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_ref().map(|s| s.as_ref())
    }

    /// Returns the subsystem part of this.
    pub fn subsystem(&self) -> Option<&str> {
        self.subsystem.as_ref().map(|s| s.as_ref())
    }

    /// Returns the name part of this.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn new(
        namespace: Option<&str>,
        subsystem: Option<&str>,
        name: &str,
    ) -> Result<Self> {
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

/// Metric kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs)]
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

pub(crate) struct MetricValue(pub f64);
impl fmt::Display for MetricValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_finite() {
            write!(f, "{}", self.0)
        } else if self.0.is_nan() {
            write!(f, "Nan")
        } else if self.0.is_sign_positive() {
            write!(f, "+Inf")
        } else {
            write!(f, "-Inf")
        }
    }
}

/// Metric families.
#[derive(Debug, Clone)]
pub struct MetricFamilies(pub(crate) Vec<MetricFamily>);
impl MetricFamilies {
    /// Consumes the `MetricFamilies` and returns the underlying vector.
    pub fn into_vec(self) -> Vec<MetricFamily> {
        self.0
    }

    /// Converts to the text format.
    pub fn to_text(&self) -> String {
        use std::fmt::Write;

        let mut buf = String::new();
        for m in &self.0 {
            write!(buf, "{}", m).expect("Never fails");
        }
        buf
    }
}
impl AsRef<[MetricFamily]> for MetricFamilies {
    fn as_ref(&self) -> &[MetricFamily] {
        &self.0
    }
}
impl IntoIterator for MetricFamilies {
    type Item = MetricFamily;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Metric family.
///
/// # References
///
/// - [metrics.proto](https://github.com/prometheus/client_model/blob/086fe7ca28bde6cec2acd5223423c1475a362858/metrics.proto#L76-%20%20L81)
#[derive(Debug, Clone)]
pub struct MetricFamily {
    name: MetricName,
    help: Option<String>,
    metrics: Metrics,
}
impl MetricFamily {
    /// Returns the name of this metric family.
    pub fn name(&self) -> &MetricName {
        &self.name
    }

    /// Returns the help of this metric family.
    pub fn help(&self) -> Option<&str> {
        self.help.as_ref().map(|h| h.as_ref())
    }

    /// Returns the kind of this metric family.
    pub fn kind(&self) -> MetricKind {
        match self.metrics {
            Metrics::Counter(_) => MetricKind::Counter,
            Metrics::Gauge(_) => MetricKind::Gauge,
            Metrics::Summary(_) => MetricKind::Summary,
            Metrics::Histogram(_) => MetricKind::Histogram,
        }
    }

    /// Returns the metrics that belongs to this family.
    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    pub(crate) fn new(metric: Metric) -> Self {
        match metric {
            Metric::Counter(m) => MetricFamily {
                name: m.metric_name().clone(),
                help: m.help().map(|h| h.to_string()),
                metrics: Metrics::Counter(vec![m]),
            },
            Metric::Gauge(m) => MetricFamily {
                name: m.metric_name().clone(),
                help: m.help().map(|h| h.to_string()),
                metrics: Metrics::Gauge(vec![m]),
            },
            Metric::Summary(m) => MetricFamily {
                name: m.metric_name().clone(),
                help: m.help().map(|h| h.to_string()),
                metrics: Metrics::Summary(vec![m]),
            },
            Metric::Histogram(m) => MetricFamily {
                name: m.metric_name().clone(),
                help: m.help().map(|h| h.to_string()),
                metrics: Metrics::Histogram(vec![m]),
            },
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
            // > HELP lines may contain any sequence of UTF-8 characters (after the metric name),
            // > but the backslash and the line-feed characters have to be escaped as \\ and \n, respectively
            write!(f, "# HELP {} ", self.name())?;
            for c in help.chars() {
                match c {
                    '\\' => write!(f, "\\\\")?,
                    '\n' => write!(f, "\\\\n")?,
                    _ => write!(f, "{}", c)?,
                }
            }
            writeln!(f, "")?;
        }
        writeln!(f, "# TYPE {} {}", self.name(), self.kind())?;
        write!(f, "{}", self.metrics)?;
        Ok(())
    }
}

/// Sequence of the same metric.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Metrics {
    Counter(Vec<Counter>),
    Gauge(Vec<Gauge>),
    Summary(Vec<Summary>),
    Histogram(Vec<Histogram>),
}
impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Metrics::Counter(ref v) => for m in v.iter() {
                writeln!(f, "{}", m)?;
            },
            Metrics::Gauge(ref v) => for m in v.iter() {
                writeln!(f, "{}", m)?;
            },
            Metrics::Summary(ref v) => for m in v.iter() {
                writeln!(f, "{}", m)?;
            },
            Metrics::Histogram(ref v) => for m in v.iter() {
                writeln!(f, "{}", m)?;
            },
        }
        Ok(())
    }
}
