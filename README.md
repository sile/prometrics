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

let counter = CounterBuilder::new("count")
    .default_registry()
    .finish()
    .unwrap();
let gauge = GaugeBuilder::new("gauge")
    .label("foo", "bar")
    .default_registry()
    .finish()
    .unwrap();

 counter.increment();
 gauge.set(12.3);

 let metrics = default_gatherer().lock().unwrap().gather();
 assert_eq!(
    metrics.to_text(),
    format!("{}\n{}\n{}\n{}\n",
            "# TYPE count counter",
            "count 1",
            "# TYPE gauge gauge",
            "gauge{foo=\"bar\"} 12.3"));
```

Benchmark
----------

```console
$ uname -a
Linux DESKTOP 4.4.0-43-Microsoft #1-Microsoft Wed Dec 31 14:42:53 PST 2014 x86_64 x86_64 x86_64 GNU/Linux

$ lscpu | grep 'Model name:'
Model name:            Intel(R) Core(TM) i7-7660U CPU @ 2.50GHz

$ cargo +nightly bench
test counter_add_float       ... bench:          10 ns/iter (+/- 0)
test counter_add_round_float ... bench:           4 ns/iter (+/- 0)
test counter_add_u64         ... bench:           4 ns/iter (+/- 0)
test counter_increment       ... bench:           4 ns/iter (+/- 0)
test gauge_set               ... bench:           4 ns/iter (+/- 0)
test histogram_observe       ... bench:          18 ns/iter (+/- 0)
test summary_observe         ... bench:         481 ns/iter (+/- 21)

test result: ok. 0 passed; 0 failed; 0 ignored; 7 measured; 0 filtered out
```

References
-----------

- [Data model](https://prometheus.io/docs/concepts/data_model/)
- [Metric types](https://prometheus.io/docs/concepts/metric_types/)
- [Writing client libraries](https://prometheus.io/docs/instrumenting/writing_clientlibs/)
- [Exposition formats](https://prometheus.io/docs/instrumenting/exposition_formats/)
