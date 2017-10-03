use std::sync::Mutex;
use std::sync::mpsc;
use trackable::error::ErrorKindExt;

use {Result, ErrorKind, Collect};
use metric::{Metric, MetricFamily};

lazy_static! {
    static ref DEFAULT_GATHERER: Mutex<MetricsGatherer> = Mutex::new(MetricsGatherer::new());
}

pub fn default_gatherer() -> &'static Mutex<MetricsGatherer> {
    &DEFAULT_GATHERER
}

pub fn default_registry() -> CollectorRegistry {
    if let Ok(gatherer) = default_gatherer().lock() {
        gatherer.registry()
    } else {
        let (tx, _) = mpsc::channel();
        CollectorRegistry { tx }
    }
}

struct Collector(Box<FnMut(&mut Vec<Metric>) -> bool + Send + 'static>);
impl Collector {
    fn collect(&mut self, metrics: &mut Vec<Metric>) -> bool {
        (self.0)(metrics)
    }
}

#[derive(Debug, Clone)]
pub struct CollectorRegistry {
    tx: mpsc::Sender<Collector>,
}
impl CollectorRegistry {
    pub fn register<C>(&self, mut collector: C) -> Result<()>
    where
        C: Collect + Send + 'static,
    {
        let f = move |metrics: &mut Vec<Metric>| if let Some(m) = collector.collect() {
            metrics.extend(m);
            true
        } else {
            false
        };
        track!(self.tx.send(Collector(Box::new(f))).map_err(|e| {
            ErrorKind::Other.cause(e.to_string())
        }))?;
        Ok(())
    }
}

pub struct MetricsGatherer {
    tx: mpsc::Sender<Collector>,
    rx: mpsc::Receiver<Collector>,
    collectors: Vec<Collector>,
}
impl MetricsGatherer {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        MetricsGatherer {
            tx,
            rx,
            collectors: Vec::new(),
        }
    }
    pub fn registry(&self) -> CollectorRegistry {
        CollectorRegistry { tx: self.tx.clone() }
    }
    pub fn gather(&mut self) -> Vec<MetricFamily> {
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
        families
    }
}
