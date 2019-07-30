// so the API is "complete" even if not all functions are used
#![allow(dead_code)]

use std::sync::atomic::{self, Ordering::Relaxed};

#[derive(Debug)]
pub struct AtomicU64(atomic::AtomicU64);

impl AtomicU64 {
    pub fn new(v: u64) -> Self {
        AtomicU64(v.into())
    }

    pub fn get(&self) -> u64 {
        self.0.load(Relaxed)
    }

    pub fn inc(&self) {
        self.add(1);
    }

    pub fn add(&self, v: u64) {
        self.0.fetch_add(v, Relaxed);
    }

    pub fn update<F>(&self, f: F)
    where
        F: Fn(u64) -> u64
    {
        let mut old = self.0.load(Relaxed);
        loop {
            let new = f(old);
            match self.0.compare_exchange_weak(old, new, Relaxed, Relaxed) {
                Ok(_) => break,
                Err(v) => old = v, // try again
            }
        }
    }

    pub fn set(&self, v: u64) {
        self.0.store(v, Relaxed);
    }
}

#[derive(Debug)]
pub struct AtomicI64(atomic::AtomicI64);

impl AtomicI64 {
    pub fn new(v: i64) -> Self {
        AtomicI64(v.into())
    }

    pub fn get(&self) -> i64 {
        self.0.load(Relaxed)
    }

    pub fn inc(&self) {
        self.add(1);
    }

    pub fn add(&self, v: i64) {
        self.0.fetch_add(v, Relaxed);
    }

    pub fn update<F>(&self, f: F)
    where
        F: Fn(i64) -> i64
    {
        let mut old = self.0.load(Relaxed);
        loop {
            let new = f(old);
            match self.0.compare_exchange_weak(old, new, Relaxed, Relaxed) {
                Ok(_) => break,
                Err(v) => old = v, // try again
            }
        }
    }

    pub fn set(&self, v: i64) {
        self.0.store(v, Relaxed);
    }
}

/// Add (and inc) is not a dedicated atomic instruction, use busy-loop
#[derive(Debug)]
pub struct AtomicF64(atomic::AtomicU64);

impl AtomicF64 {
    pub fn new(v: f64) -> Self {
        AtomicF64(v.to_bits().into())
    }

    pub fn get(&self) -> f64 {
        f64::from_bits(self.0.load(Relaxed))
    }

    pub fn inc(&self) {
        self.add(1.0);
    }

    pub fn add(&self, v: f64) {
        self.update(|old| old + v);
    }

    pub fn update<F>(&self, f: F)
    where
        F: Fn(f64) -> f64
    {
        let mut old = self.0.load(Relaxed);
        loop {
            let new = f(f64::from_bits(old)).to_bits();
            match self.0.compare_exchange_weak(old, new, Relaxed, Relaxed) {
                Ok(_) => break,
                Err(v) => old = v, // try again
            }
        }
    }

    pub fn set(&self, v: f64) {
        self.0.store(v.to_bits(), Relaxed);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn atomic_f64_works() {
        let value = AtomicF64::new(0.0);
        assert_eq!(value.get(), 0.0);

        value.set(123456789.0);
        assert_eq!(value.get(), 123456789.0);

        value.update(|v| v + 1.0);
        assert_eq!(value.get(), 123456790.0);
    }
}
