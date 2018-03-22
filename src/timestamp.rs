//! Unix timestamp.
use std;
use std::ops::Deref;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use atomic::AtomicI64;

const NO_VALUE: i64 = std::i64::MIN;

/// Unix timestamp in milliseconds.
#[derive(Debug)]
pub struct Timestamp(AtomicI64);
impl Timestamp {
    /// Returns the value of this timestamp.
    pub fn get(&self) -> Option<i64> {
        let v = self.0.get();
        if v == NO_VALUE {
            None
        } else {
            Some(v)
        }
    }

    pub(crate) fn new() -> Self {
        Timestamp(AtomicI64::new(NO_VALUE))
    }
    fn clear(&self) {
        self.0.set(NO_VALUE);
    }
    fn set(&self, timestamp: i64) {
        assert_ne!(timestamp, NO_VALUE);
        self.0.set(timestamp);
    }
    fn set_time(&self, time: SystemTime) {
        fn to_millis(d: Duration) -> i64 {
            d.as_secs() as i64 * 1000 + i64::from(d.subsec_nanos()) / 1000 / 1000
        }
        if let Ok(duration) = time.duration_since(UNIX_EPOCH) {
            self.set(to_millis(duration));
        } else {
            let duration = UNIX_EPOCH.duration_since(time).expect("Never fails");
            self.set(-to_millis(duration));
        }
    }
    fn set_now(&self) {
        self.set_time(SystemTime::now());
    }
}

/// Mutable variant of `Timestamp`.
#[derive(Debug)]
pub struct TimestampMut<'a>(&'a Timestamp);
impl<'a> TimestampMut<'a> {
    /// Sets the value of this timestamp to `timestamp`.
    pub fn set(&mut self, timestamp: i64) {
        self.0.set(timestamp)
    }

    /// Sets the value of this timestamp to the current unixtime in milliseconds.
    pub fn set_now(&mut self) {
        self.0.set_now()
    }

    /// Sets the value of this timestamp to `time`.
    pub fn set_time(&mut self, time: SystemTime) {
        self.0.set_time(time)
    }

    /// Clears the value of this timestamp.
    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub(crate) fn new(inner: &'a Timestamp) -> Self {
        TimestampMut(inner)
    }
}
impl<'a> Deref for TimestampMut<'a> {
    type Target = Timestamp;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

pub(crate) fn now_unixtime_seconds() -> f64 {
    let now = SystemTime::now();
    if let Ok(d) = now.duration_since(UNIX_EPOCH) {
        duration_to_unixtime_seconds(d)
    } else {
        let d = UNIX_EPOCH.duration_since(now).expect("Never fails");
        -duration_to_unixtime_seconds(d)
    }
}

pub(crate) fn duration_to_unixtime_seconds(d: Duration) -> f64 {
    d.as_secs() as f64 + f64::from(d.subsec_nanos()) / 1_000_000_000.0
}
