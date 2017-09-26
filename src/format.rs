use std::io::Write;

use Result;
use metric::Metric;

pub trait Format {
    fn format<W: Write>(&self, writer: &mut W, metric: &Metric) -> Result<()>;
}

#[derive(Debug)]
pub struct MetricWriter<W, F> {
    writer: W,
    formatter: F,
}
impl<W, F> MetricWriter<W, F>
where
    W: Write,
    F: Format,
{
    pub fn new(writer: W, formatter: F) -> Self {
        MetricWriter { writer, formatter }
    }
    pub fn write(&mut self, metric: &Metric) -> Result<()> {
        track!(self.formatter.format(&mut self.writer, metric))?;
        Ok(())
    }
    pub fn into_writer(self) -> W {
        self.writer
    }
}
