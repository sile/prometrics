use {Result, ErrorKind};

#[derive(Debug, Clone)]
pub struct Quantile {
    quantile: f64,
    value: f64,
}
impl Quantile {
    pub fn quantile(&self) -> f64 {
        self.quantile
    }
    pub fn value(&self) -> f64 {
        self.value
    }

    pub(crate) fn new(quantile: f64, value: f64) -> Result<Self> {
        track_assert!(
            0.0 <= quantile && quantile <= 1.0,
            ErrorKind::InvalidInput,
            "quantile:{}",
            quantile
        );
        Ok(Quantile { quantile, value })
    }
    pub(crate) fn with_value(&self, value: f64) -> Self {
        let mut this = self.clone();
        this.value = value;
        this
    }
}
