use Metric;

pub trait Collect {
    type Metrics: Iterator<Item = Metric>;
    fn collect(&mut self) -> Option<Self::Metrics>;
}
