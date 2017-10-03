use std::vec;
#[cfg(target_os = "linux")]
use libc;
#[cfg(target_os = "linux")]
use procinfo;

use Collect;
use metric::Metric;
use metrics::{CounterBuilder, GaugeBuilder};

lazy_static! {
    static ref CLK_TCK: f64 = { unsafe { libc::sysconf(libc::_SC_CLK_TCK) as f64 } };
    static ref PAGESIZE: usize = { unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize } };
}

/// Process metrics collector.
///
/// # Notice
///
/// On non Linux platforms, the `collect` method always returns `None`.
///
/// # Reference
///
/// - [process metrics](https://prometheus.io/docs/instrumenting/writing_clientlibs/#process-metrics)
#[derive(Debug, Default)]
pub struct ProcessMetricsCollector(());
impl ProcessMetricsCollector {
    /// Makes a new `ProcessMetricsCollector` instance.
    pub fn new() -> Self {
        ProcessMetricsCollector(())
    }
}
impl Collect for ProcessMetricsCollector {
    type Metrics = vec::IntoIter<Metric>;
    #[cfg(target_os = "linux")]
    fn collect(&mut self) -> Option<Self::Metrics> {
        let mut metrics = Vec::new();

        if let Ok(limits) = procinfo::pid::limits_self() {
            if let Some(fds) = limits.max_open_files.soft {
                metrics.push(gauge("max_fds", fds as f64));
            }
        }
        if let Ok(status) = procinfo::pid::status_self() {
            metrics.push(gauge("open_fds", status.fd_allocated as f64));
        }
        if let Ok(stat) = procinfo::pid::stat_self() {
            metrics.push(counter(
                "cpu_seconds_total",
                (stat.utime + stat.stime) as f64 / *CLK_TCK,
            ));
            metrics.push(gauge(
                "start_time_seconds",
                stat.start_time as f64 / *CLK_TCK,
            ));
            metrics.push(gauge(
                "virtual_memory_bytes",
                (stat.vsize * *PAGESIZE) as f64,
            ));
            metrics.push(gauge(
                "resident_memory_bytes",
                (stat.rss * *PAGESIZE) as f64,
            ));
        }

        Some(metrics.into_iter())
    }
    #[cfg(not(target_os = "linux"))]
    fn collect(&mut self) -> Option<Self::Metrics> {
        None
    }
}

fn counter(name: &str, value: f64) -> Metric {
    let mut counter = CounterBuilder::new(name)
        .namespace("process")
        .finish()
        .expect("Never fails");
    let _ = counter.add(value);
    counter.into()
}

fn gauge(name: &str, value: f64) -> Metric {
    let mut gauge = GaugeBuilder::new(name)
        .namespace("process")
        .finish()
        .expect("Never fails");
    gauge.set(value);
    gauge.into()
}
