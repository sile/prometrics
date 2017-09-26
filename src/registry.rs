use std::sync::mpsc;
use std::mem;

use {Result, Error};
use collector::{Collector, BoxCollector};
use metric::Metric;

#[derive(Clone)]
pub struct CollectorRegistry {
    tx: mpsc::Sender<BoxCollector>,
}
impl CollectorRegistry {
    pub fn register<C>(&self, collector: C) -> Result<()>
    where
        C: Collector + Send + 'static,
    {
        track!(self.tx.send(Box::new(collector)).map_err(Error::from))?;
        Ok(())
    }
}

pub struct MetricsGatherer {
    tx: mpsc::Sender<BoxCollector>,
    rx: mpsc::Receiver<BoxCollector>,
    collectors: Vec<BoxCollector>,
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
    pub fn gather(&mut self) -> Metrics {
        while let Ok(collector) = self.rx.try_recv() {
            self.collectors.push(collector);
        }
        Metrics {
            collectors: &mut self.collectors,
            metrics: &[],
            collector_index: 0,
            metric_index: 0,
        }
    }
}

pub struct Metrics<'a> {
    collectors: &'a mut Vec<BoxCollector>,
    metrics: &'a [Metric],
    collector_index: usize,
    metric_index: usize,
}
impl<'a> Iterator for Metrics<'a> {
    type Item = &'a Metric;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(metric) = self.metrics.get(self.metric_index) {
                self.metric_index += 1;
                return Some(metric);
            } else if self.collector_index == self.collectors.len() {
                return None;
            } else if let Some(metrics) = self.collectors[self.collector_index].collect() {
                self.metrics = unsafe { mem::transmute(metrics) };
                self.metric_index = 0;
                self.collector_index += 1;
                continue;
            }
            self.collectors.swap_remove(self.collector_index);
        }
    }
}
