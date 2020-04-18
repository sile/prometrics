use std::cmp;
use std::fmt;

use bucket::AggregatedCumulativeBuckets;
use label::Labels;
use metric::{MetricName, MetricValue};
use metrics::{Counter, Gauge, Histogram, Summary};
use quantile::Quantile;
use timestamp::Timestamp;

/// A metric for aggregating counters that have the same name and labels.
#[derive(Debug, Clone)]
pub struct AggregatedCounter {
    inner: Counter,
    timestamp: Option<i64>,
    value: f64,
}
impl AggregatedCounter {
    /// Returns the name of this metric.
    pub fn metric_name(&self) -> &MetricName {
        self.inner.metric_name()
    }

    /// Returns the labels of this metric.
    pub fn labels(&self) -> &Labels {
        self.inner.labels()
    }

    /// Returns the latest timestamp among the counters in this aggregation.
    pub fn timestamp(&self) -> Timestamp {
        Timestamp::from_value(self.timestamp)
    }

    /// Returns the sum of the value of the counters in this aggregation.
    pub fn value(&self) -> f64 {
        self.value
    }

    pub(crate) fn new(counter: Counter) -> Self {
        let value = counter.value();
        let timestamp = counter.timestamp().get();
        AggregatedCounter {
            inner: counter,
            timestamp,
            value,
        }
    }

    pub(crate) fn try_merge(&mut self, other: &Self) -> bool {
        let is_same_metric = self.metric_name() == other.metric_name()
            && self.labels().iter().eq(other.labels().iter());
        if is_same_metric {
            self.value += other.value;
            self.timestamp = cmp::max(self.timestamp, other.timestamp);
            true
        } else {
            false
        }
    }
}
impl fmt::Display for AggregatedCounter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.metric_name())?;
        if !self.labels().is_empty() {
            write!(f, "{}", self.labels())?;
        }
        write!(f, " {}", MetricValue(self.value()))?;
        if let Some(timestamp) = self.timestamp {
            write!(f, " {}", timestamp)?;
        }
        Ok(())
    }
}

/// A metric for aggregating gauges that have the same name and labels.
#[derive(Debug, Clone)]
pub struct AggregatedGauge {
    inner: Gauge,
    timestamp: Option<i64>,
    value: f64,
}
impl AggregatedGauge {
    /// Returns the name of this metric.
    pub fn metric_name(&self) -> &MetricName {
        self.inner.metric_name()
    }

    /// Returns the labels of this metric.
    pub fn labels(&self) -> &Labels {
        self.inner.labels()
    }

    /// Returns the latest timestamp among the counters in this aggregation.
    pub fn timestamp(&self) -> Timestamp {
        Timestamp::from_value(self.timestamp)
    }

    /// Returns the sum of the value of the gauges in this aggregation.
    pub fn value(&self) -> f64 {
        self.value
    }

    pub(crate) fn new(gauge: Gauge) -> Self {
        let value = gauge.value();
        let timestamp = gauge.timestamp().get();
        AggregatedGauge {
            inner: gauge,
            timestamp,
            value,
        }
    }

    pub(crate) fn try_merge(&mut self, other: &Self) -> bool {
        let is_same_metric = self.metric_name() == other.metric_name()
            && self.labels().iter().eq(other.labels().iter());
        if is_same_metric {
            self.value += other.value;
            self.timestamp = cmp::max(self.timestamp, other.timestamp);
            true
        } else {
            false
        }
    }
}
impl fmt::Display for AggregatedGauge {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.metric_name())?;
        if !self.labels().is_empty() {
            write!(f, "{}", self.labels())?;
        }
        write!(f, " {}", MetricValue(self.value()))?;
        if let Some(timestamp) = self.timestamp {
            write!(f, " {}", timestamp)?;
        }
        Ok(())
    }
}

/// A metric for aggregating histograms that have the same name and labels.
#[derive(Debug, Clone)]
pub struct AggregatedHistogram {
    inners: Vec<Histogram>,
}
impl AggregatedHistogram {
    /// Returns the name of this metric.
    pub fn metric_name(&self) -> &MetricName {
        self.inners[0].metric_name()
    }

    /// Returns the labels of this metric.
    pub fn labels(&self) -> &Labels {
        self.inners[0].labels()
    }

    /// Returns the latest timestamp among the histograms in this aggregation.
    pub fn timestamp(&self) -> Timestamp {
        Timestamp::from_value(
            self.inners
                .iter()
                .map(|h| h.timestamp().get())
                .max()
                .and_then(|t| t),
        )
    }

    /// Returns the cumulative buckets of this aggregation.
    pub fn cumulative_buckets(&self) -> AggregatedCumulativeBuckets {
        AggregatedCumulativeBuckets::new(&self.inners)
    }

    /// Returns the sum of the observation counts in this aggregation.
    pub fn count(&self) -> u64 {
        self.inners.iter().map(|h| h.count()).sum()
    }

    /// Returns the sum of the observed values in this aggregation.
    pub fn sum(&self) -> f64 {
        self.inners.iter().map(|h| h.sum()).sum()
    }

    pub(crate) fn new(histogram: Histogram) -> Self {
        AggregatedHistogram {
            inners: vec![histogram],
        }
    }

    pub(crate) fn try_merge(&mut self, other: &Self) -> bool {
        let is_same_metric = self.metric_name() == other.metric_name()
            && self.labels().iter().eq(other.labels().iter());
        if is_same_metric {
            self.inners.extend_from_slice(&other.inners);
            true
        } else {
            false
        }
    }
}
impl fmt::Display for AggregatedHistogram {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let labels = if !self.labels().is_empty() {
            self.labels().to_string()
        } else {
            "".to_string()
        };
        let timestamp = if let Some(t) = self.timestamp().get() {
            format!(" {}", t)
        } else {
            "".to_string()
        };

        for bucket in self.cumulative_buckets() {
            write!(
                f,
                "{}_bucket{{le=\"{}\"",
                self.metric_name(),
                MetricValue(bucket.upper_bound())
            )?;
            for label in self.labels().iter() {
                write!(f, ",{}={:?}", label.name(), label.value())?;
            }
            writeln!(f, "}} {}{}", bucket.cumulative_count(), timestamp)?;
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

/// A metric for aggregating summaries that have the same name and labels.
#[derive(Debug, Clone)]
pub struct AggregatedSummary {
    inners: Vec<Summary>,
}
impl AggregatedSummary {
    /// Returns the name of this metric.
    pub fn metric_name(&self) -> &MetricName {
        self.inners[0].metric_name()
    }

    /// Returns the labels of this metric.
    pub fn labels(&self) -> &Labels {
        self.inners[0].labels()
    }

    /// Returns the latest timestamp among the summaries in this aggregation.
    pub fn timestamp(&self) -> Timestamp {
        Timestamp::from_value(
            self.inners
                .iter()
                .map(|h| h.timestamp().get())
                .max()
                .and_then(|t| t),
        )
    }

    /// Returns the sum of the observation counts in this aggregation.
    pub fn count(&self) -> u64 {
        self.inners.iter().map(|h| h.count()).sum()
    }

    /// Returns the sum of the observed values in this aggregation.
    pub fn sum(&self) -> f64 {
        self.inners.iter().map(|h| h.sum()).sum()
    }

    /// Calculates and returns the quantile-value pairs of this aggregation.
    pub fn quantiles(&self) -> Vec<(Quantile, f64)> {
        let mut aggregated_samples = Vec::new();
        for summary in &self.inners {
            summary.with_current_samples(|_, samples| {
                aggregated_samples.extend(samples.iter().map(|&(_, v)| v).filter(|v| !v.is_nan()));
            });
        }
        aggregated_samples.sort_by(|a, b| a.partial_cmp(b).expect("Never fails"));

        if aggregated_samples.is_empty() {
            return Vec::new();
        }
        let count = aggregated_samples.len();

        let mut quantiles = self
            .inners
            .iter()
            .flat_map(|s| s.quantiles_without_values().iter())
            .cloned()
            .collect::<Vec<_>>();
        quantiles.sort();
        quantiles.dedup();
        quantiles
            .iter()
            .map(|&quantile| {
                let index = cmp::min(count, (quantile.as_f64() * count as f64).floor() as usize);
                (quantile, aggregated_samples[index])
            })
            .collect()
    }

    pub(crate) fn new(summary: Summary) -> Self {
        AggregatedSummary {
            inners: vec![summary],
        }
    }

    pub(crate) fn try_merge(&mut self, other: &Self) -> bool {
        let is_same_metric = self.metric_name() == other.metric_name()
            && self.labels().iter().eq(other.labels().iter());
        if is_same_metric {
            self.inners.extend_from_slice(&other.inners);
            true
        } else {
            false
        }
    }
}
impl fmt::Display for AggregatedSummary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let labels = if !self.labels().is_empty() {
            self.labels().to_string()
        } else {
            "".to_string()
        };
        let timestamp = if let Some(t) = self.timestamp().get() {
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
