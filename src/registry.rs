use std::fmt;
use std::sync::Mutex;
use std::sync::mpsc;

use Collect;
use metric::{Metric, MetricFamilies, MetricFamily};

lazy_static! {
    static ref DEFAULT_GATHERER: Mutex<Gatherer> = Mutex::new(Gatherer::new());
}

/// Returns the global default `Gatherer`.
pub fn default_gatherer() -> &'static Mutex<Gatherer> {
    &DEFAULT_GATHERER
}

/// Returns the global default `Registry`.
pub fn default_registry() -> Registry {
    if let Ok(gatherer) = default_gatherer().lock() {
        gatherer.registry()
    } else {
        let (tx, _) = mpsc::channel();
        Registry { tx }
    }
}

/// Collector registry.
#[derive(Debug, Clone)]
pub struct Registry {
    tx: mpsc::Sender<Collector>,
}
impl Registry {
    /// Registers a collector.
    ///
    /// If `collector.collect()` returns `None`, it will be deregistered from this.
    pub fn register<C>(&self, mut collector: C)
    where
        C: Collect + Send + 'static,
    {
        let f = move |metrics: &mut Vec<Metric>| {
            if let Some(m) = collector.collect() {
                metrics.extend(m);
                true
            } else {
                false
            }
        };
        let _ = self.tx.send(Collector(Box::new(f)));
    }
}

struct Collector(Box<FnMut(&mut Vec<Metric>) -> bool + Send + 'static>);
impl Collector {
    fn collect(&mut self, metrics: &mut Vec<Metric>) -> bool {
        (self.0)(metrics)
    }
}
impl fmt::Debug for Collector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Collector(_)")
    }
}

/// Metrics gatherer.
///
/// This can gather metrics that registered to registries which associated with the gatherer.
#[derive(Debug)]
pub struct Gatherer {
    tx: mpsc::Sender<Collector>,
    rx: mpsc::Receiver<Collector>,
    collectors: Vec<Collector>,
}
#[cfg_attr(feature = "cargo-clippy", allow(new_without_default))]
impl Gatherer {
    /// Makes a new `Gatherer` instance.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Gatherer {
            tx,
            rx,
            collectors: Vec::new(),
        }
    }

    /// Returns a `Registry` associated with this gatherer.
    pub fn registry(&self) -> Registry {
        Registry {
            tx: self.tx.clone(),
        }
    }

    /// Gathers metrics.
    pub fn gather(&mut self) -> MetricFamilies {
        while let Ok(collector) = self.rx.try_recv() {
            self.collectors.push(collector);
        }

        let mut metrics = Vec::new();
        let mut i = 0;
        while i < self.collectors.len() {
            if self.collectors[i].collect(&mut metrics) {
                i += 1;
            } else {
                self.collectors.swap_remove(i);
            }
        }
        metrics.sort_by(|a, b| (a.name(), a.kind()).cmp(&(b.name(), b.kind())));

        let mut families: Vec<MetricFamily> = Vec::new();
        for metric in metrics {
            if !families.last().map_or(false, |f| f.same_family(&metric)) {
                families.push(MetricFamily::new(metric));
            } else {
                families.last_mut().unwrap().push(metric);
            }
        }
        MetricFamilies(families)
    }
}
