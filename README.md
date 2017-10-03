prometrics
===========

[![Crates.io: prometrics](http://meritbadge.herokuapp.com/prometrics)](https://crates.io/crates/prometrics)
[![Documentation](https://docs.rs/prometrics/badge.svg)](https://docs.rs/prometrics)
[![Build Status](https://travis-ci.org/sile/prometrics.svg?branch=master)](https://travis-ci.org/sile/prometrics)
[![Code Coverage](https://codecov.io/gh/sile/prometrics/branch/master/graph/badge.svg)](https://codecov.io/gh/sile/prometrics/branch/master)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Rust client library for exposing [prometheus][prometheus] metrics.

[prometheus]: https://prometheus.io/

[Documentation](https://docs.rs/prometrics)

Examples
--------

```rust
use prometrics::default_gatherer;
use prometrics::metrics::{CounterBuilder, GaugeBuilder};

let mut counter = CounterBuilder::new("count")
    .default_registry()
    .finish()
    .unwrap();
let mut gauge = GaugeBuilder::new("gauge")
    .label("foo", "bar")
    .default_registry()
    .finish()
    .unwrap();

 counter.increment();
 gauge.set(12.3);

 let metrics = default_gatherer().lock().unwrap().gather();
 assert_eq!(
    metrics
        .into_iter()
        .map(|m| format!("\n{}", m))
        .collect::<Vec<_>>()
        .join(""),
    r#"
# TYPE count counter
count 1

# TYPE gauge gauge
gauge{foo="bar"} 12.3
"#
```

References
-----------

- [Data model](https://prometheus.io/docs/concepts/data_model/)
- [Metric types](https://prometheus.io/docs/concepts/metric_types/)
- [Writing client libraries](https://prometheus.io/docs/instrumenting/writing_clientlibs/)
- [Exposition formats](https://prometheus.io/docs/instrumenting/exposition_formats/)
