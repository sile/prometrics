//! Summary quantile.
use {Result, ErrorKind};

/// Summary quantile.
#[derive(Debug, Clone, Copy)]
pub struct Quantile(f64);
impl Quantile {
    /// Makes a new `Quantile` instance.
    ///
    /// # Errors
    ///
    /// If `quantile` is not in the range `0.0...1.0`,
    /// this function will return `ErrorKind::InvalidInput` error.
    pub fn new(quantile: f64) -> Result<Self> {
        track_assert!(
            0.0 <= quantile && quantile <= 1.0,
            ErrorKind::InvalidInput,
            "quantile:{}",
            quantile
        );
        Ok(Quantile(quantile))
    }

    /// Converts `Quantile` to `f64`.
    pub fn as_f64(&self) -> f64 {
        self.0
    }
}
