//! Bucket for [histogram][histogram] metric.
//!
//! [histogram]: https://prometheus.io/docs/concepts/metric_types/#histogram
use std;
use std::iter::Peekable;
use std::slice;

use atomic::AtomicU64;
use metrics::Histogram;
use {ErrorKind, Result};

/// A bucket in which a [histogram][histogram] counts samples.
///
/// Note that this bucket is not cumulative.
///
/// [histogram]: https://prometheus.io/docs/concepts/metric_types/#histogram
#[derive(Debug)]
pub struct Bucket {
    count: AtomicU64,
    upper_bound: f64,
}
impl Bucket {
    /// Returns the count of samples in this bucket.
    #[inline]
    pub fn count(&self) -> u64 {
        self.count.get()
    }

    /// Returns the upper bound of this bucket.
    ///
    /// This method never return a NaN value.
    #[inline]
    pub fn upper_bound(&self) -> f64 {
        self.upper_bound
    }

    pub(crate) fn new(upper_bound: f64) -> Result<Self> {
        track_assert!(!upper_bound.is_nan(), ErrorKind::InvalidInput);
        Ok(Bucket {
            count: AtomicU64::new(0),
            upper_bound,
        })
    }

    #[inline]
    pub(crate) fn increment(&self) {
        self.count.inc();
    }
}

/// Cumulative bucket.
#[derive(Debug, Clone)]
pub struct CumulativeBucket {
    cumulative_count: u64,
    upper_bound: f64,
}
impl CumulativeBucket {
    /// Returns the cumulative count of samples.
    pub fn cumulative_count(&self) -> u64 {
        self.cumulative_count
    }

    /// Returns the upper bound of this bucket.
    ///
    /// This method never return a NaN value.
    pub fn upper_bound(&self) -> f64 {
        self.upper_bound
    }
}

/// An iterator which iterates cumulative buckets in a histogram.
#[derive(Debug)]
pub struct CumulativeBuckets<'a> {
    cumulative_count: u64,
    iter: slice::Iter<'a, Bucket>,
}
impl<'a> CumulativeBuckets<'a> {
    pub(crate) fn new(buckets: &'a [Bucket]) -> Self {
        CumulativeBuckets {
            cumulative_count: 0,
            iter: buckets.iter(),
        }
    }
}
impl<'a> Iterator for CumulativeBuckets<'a> {
    type Item = CumulativeBucket;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|b| {
            self.cumulative_count += b.count();
            CumulativeBucket {
                cumulative_count: self.cumulative_count,
                upper_bound: b.upper_bound(),
            }
        })
    }
}

/// An iterator which iterates cumulative buckets in an aggregation of histograms.
#[derive(Debug)]
pub struct AggregatedCumulativeBuckets<'a> {
    cumulative_count: u64,
    iters: Vec<Peekable<slice::Iter<'a, Bucket>>>,
}
impl<'a> AggregatedCumulativeBuckets<'a> {
    pub(crate) fn new(histograms: &'a [Histogram]) -> Self {
        AggregatedCumulativeBuckets {
            cumulative_count: 0,
            iters: histograms
                .iter()
                .map(|h| h.buckets().iter().peekable())
                .collect(),
        }
    }
}
impl<'a> Iterator for AggregatedCumulativeBuckets<'a> {
    type Item = CumulativeBucket;
    fn next(&mut self) -> Option<Self::Item> {
        let mut min = std::f64::INFINITY;
        let mut i = 0;
        while i < self.iters.len() {
            if let Some(bound) = self.iters[i].peek().map(|b| b.upper_bound()) {
                if bound < min {
                    min = bound;
                }
                i += 1;
            } else {
                let _ = self.iters.swap_remove(i);
            }
        }
        if self.iters.is_empty() {
            return None;
        }

        for buckets in &mut self.iters {
            let upper_bound = buckets.peek().expect("Never fails").upper_bound();
            if min.is_infinite() || (upper_bound - min).abs() < std::f64::EPSILON {
                let bucket = buckets.next().expect("Never fails");
                self.cumulative_count += bucket.count();
            }
        }

        Some(CumulativeBucket {
            cumulative_count: self.cumulative_count,
            upper_bound: min,
        })
    }
}
