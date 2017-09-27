use std;
use std::ops::Deref;
use std::time::{SystemTime, Duration, UNIX_EPOCH};

use atomic;

#[derive(Debug)]
pub struct Timestamp(atomic::Atomic64<i64>);
impl Timestamp {
    pub fn new() -> Self {
        Timestamp(atomic::Atomic64::new(std::i64::MIN))
    }
    pub fn get(&self) -> Option<i64> {
        let v = self.0.get();
        if v == std::i64::MIN { None } else { Some(v) }
    }
    fn clear(&self) {
        self.0.set(std::i64::MIN);
    }
    fn set(&self, timestamp: i64) {
        assert_ne!(timestamp, std::i64::MIN);
        self.0.set(timestamp);
    }
    fn set_time(&self, time: SystemTime) {
        fn to_millis(d: Duration) -> i64 {
            d.as_secs() as i64 * 1000 + d.subsec_nanos() as i64 / 1000 / 1000
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

#[derive(Debug)]
pub struct TimestampMut<'a>(pub(crate) &'a Timestamp); // TODO
impl<'a> TimestampMut<'a> {
    pub fn clear(&mut self) {
        self.0.clear()
    }
    pub fn set(&mut self, timestamp: i64) {
        self.0.set(timestamp)
    }
    pub fn set_time(&mut self, time: SystemTime) {
        self.0.set_time(time)
    }
    pub fn set_now(&mut self) {
        self.0.set_now()
    }
}
impl<'a> Deref for TimestampMut<'a> {
    type Target = Timestamp;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
