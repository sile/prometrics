use Metric;

/// This trait allows for collecting metrics.
pub trait Collect {
    /// An iterator over collected metrics.
    type Metrics: Iterator<Item = Metric>;

    /// Collects metrics.
    ///
    /// If there are no more metrics to collect, this method will return `None`.
    fn collect(&mut self) -> Option<Self::Metrics>;
}
