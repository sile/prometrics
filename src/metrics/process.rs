use std::time::SystemTime;
#[cfg(target_os = "linux")]
use std::time::UNIX_EPOCH;
use std::vec;
#[cfg(target_os = "linux")]
use libc;
#[cfg(target_os = "linux")]
use procinfo;

use Collect;
use metric::Metric;
#[cfg(target_os = "linux")]
use metrics::{CounterBuilder, GaugeBuilder};

#[cfg(target_os = "linux")]
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
///
/// # Examples
///
/// ```
/// use prometrics::{default_gatherer, default_registry};
/// use prometrics::metrics::ProcessMetricsCollector;
///
/// // Register
/// default_registry().register(ProcessMetricsCollector::new());
///
/// // Gather
/// let _metrics = default_gatherer().lock().unwrap().gather();
/// ```
#[derive(Debug)]
pub struct ProcessMetricsCollector {
    start_time: SystemTime,
}
impl ProcessMetricsCollector {
    /// Makes a new `ProcessMetricsCollector` instance.
    pub fn new() -> Self {
        ProcessMetricsCollector { start_time: SystemTime::now() }
    }
}
impl Default for ProcessMetricsCollector {
    fn default() -> Self {
        Self::new()
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
            metrics.push(gauge("open_fds", f64::from(status.fd_allocated)));
        }
        if let Ok(stat) = procinfo::pid::stat_self() {
            metrics.push(counter(
                "cpu_seconds_total",
                (stat.utime + stat.stime) as f64 / *CLK_TCK,
            ));
            if let Ok(start_time) = self.start_time.duration_since(UNIX_EPOCH) {
                metrics.push(gauge("start_time_seconds", start_time.as_secs() as f64));
            }
            metrics.push(gauge("threads_total", f64::from(stat.num_threads)));
            metrics.push(gauge("virtual_memory_bytes", stat.vsize as f64));
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

#[cfg(target_os = "linux")]
fn counter(name: &str, value: f64) -> Metric {
    let mut counter = CounterBuilder::new(name)
        .namespace("process")
        .finish()
        .expect("Never fails");
    let _ = counter.add(value);
    counter.into()
}

#[cfg(target_os = "linux")]
fn gauge(name: &str, value: f64) -> Metric {
    let mut gauge = GaugeBuilder::new(name)
        .namespace("process")
        .finish()
        .expect("Never fails");
    gauge.set(value);
    gauge.into()
}
