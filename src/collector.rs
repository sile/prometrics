use metric::Metric;

pub trait Collector {
    fn collect(&mut self) -> Option<&[Metric]>;
}

pub type BoxCollector = Box<Collector + Send + 'static>;
