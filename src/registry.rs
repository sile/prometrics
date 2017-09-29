use std::sync::mpsc;

use {Result, Error, Collector};
use metric::MetricFamily;

pub fn default_registry() -> CollectorRegistry {
    unimplemented!()
}

#[derive(Debug, Clone)]
pub struct CollectorRegistry {
    tx: mpsc::Sender<Box<Collector + Send + 'static>>,
}
impl CollectorRegistry {
    pub fn register<C>(&self, collector: C) -> Result<()>
    where
        C: Collector + Send + 'static,
    {
        track!(self.tx.send(Box::new(collector)).map_err(Error::from))
    }
}

pub struct MetricsGatherer {
    tx: mpsc::Sender<Box<Collector + Send + 'static>>,
    rx: mpsc::Receiver<Box<Collector + Send + 'static>>,
    collectors: Vec<Box<Collector + Send + 'static>>,
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
            if let Some(m) = self.collectors[i].collect() {
                metrics.extend(m);
                i += 1;
            } else {
                self.collectors.swap_remove(i);
            }
        }
        metrics.sort_by(|a, b| (a.name(), a.kind()).cmp(&(b.name(), b.kind())));

        let mut families: Vec<MetricFamily> = Vec::new();
        for metric in metrics.into_iter() {
            if !families.last().map_or(false, |f| f.same_family(&metric)) {
                families.push(MetricFamily::new(metric));
            } else {
                families.last_mut().unwrap().push(metric);
            }
        }
        families
    }
}
