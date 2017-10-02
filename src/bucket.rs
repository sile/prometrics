//! Bucket for [histogram][histogram] metric.
//!
//! [histogram]: https://prometheus.io/docs/concepts/metric_types/#histogram
use std;
use std::slice;

use {Result, ErrorKind};
use atomic::AtomicU64;

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
    pub fn count(&self) -> u64 {
        self.count.get()
    }

    /// Returns the upper bound of this bucket.
    ///
    /// This method never return a NaN value.
    pub fn upper_bound(&self) -> f64 {
        self.upper_bound
    }

    pub(crate) fn new(upper_bound: f64) -> Result<Self> {
        track_assert_ne!(upper_bound, std::f64::NAN, ErrorKind::InvalidInput);
        Ok(Bucket {
            count: AtomicU64::new(0),
            upper_bound,
        })
    }
    pub(crate) fn increment(&self) {
        self.count.inc();
    }
}

/// Cumulative bucket.
#[derive(Debug)]
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
