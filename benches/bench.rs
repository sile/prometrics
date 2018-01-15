#![feature(test)]
extern crate prometrics;
extern crate test;

use std::time::Duration;
use prometrics::metrics::{Counter, Gauge, HistogramBuilder, Summary};

#[bench]
fn counter_increment(b: &mut test::Bencher) {
    let counter = Counter::new("bench").unwrap();
    b.iter(|| {
        counter.increment();
    })
}

#[bench]
fn counter_add_float(b: &mut test::Bencher) {
    let counter = Counter::new("bench").unwrap();
    b.iter(|| {
        let _ = counter.add(3.3);
    })
}

#[bench]
fn counter_add_round_float(b: &mut test::Bencher) {
    let counter = Counter::new("bench").unwrap();
    b.iter(|| {
        let _ = counter.add(3.0);
    })
}

#[bench]
fn counter_add_u64(b: &mut test::Bencher) {
    let counter = Counter::new("bench").unwrap();
    b.iter(|| {
        counter.add_u64(3);
    })
}

#[bench]
fn gauge_set(b: &mut test::Bencher) {
    let gauge = Gauge::new("bench").unwrap();
    b.iter(|| {
        gauge.set(3.3);
    })
}

#[bench]
fn histogram_observe(b: &mut test::Bencher) {
    let histogram = HistogramBuilder::with_linear_buckets("bench", 0.0, 1.0, 10)
        .finish()
        .unwrap();
    b.iter(|| {
        histogram.observe(3.3);
    })
}

#[bench]
fn summary_observe(b: &mut test::Bencher) {
    let summary = Summary::new("bench", Duration::from_millis(10)).unwrap();
    b.iter(|| {
        summary.observe(3.3);
    })
}
