use std::slice;

use atomic;

#[derive(Debug)]
pub struct Bucket {
    count: atomic::Atomic64<u64>,
    upper_bound: f64,
}
impl Bucket {
    pub fn new(upper_bound: f64) -> Self {
        Bucket {
            count: atomic::Atomic64::new(0),
            upper_bound,
        }
    }
    pub(crate) fn inc(&self) {
        self.count.inc();
    }
    pub fn count(&self) -> u64 {
        self.count.get()
    }
    pub fn upper_bound(&self) -> f64 {
        self.upper_bound
    }
}

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
    type Item = (u64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|b| {
            self.cumulative_count += b.count();
            (self.cumulative_count, b.upper_bound())
        })
    }
}
