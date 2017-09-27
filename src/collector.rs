use Metric;

pub trait Collector {
    fn collect(&mut self) -> Option<Box<Iterator<Item = Metric>>>;
}
