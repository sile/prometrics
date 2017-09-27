use std;
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::sync::Arc;
use std::slice;

pub type AtomicF64 = atomic::Atomic64<f64>;

#[derive(Debug, Clone)]
pub struct Label {
    pub name: String,
    pub value: String,
}
impl Label {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn value(&self) -> &str {
        &self.value
    }
}

pub struct Labels<'a> {
    pub(crate) static_labels: &'a Vec<Label>,
    pub(crate) dynamic_labels: Arc<Vec<Label>>,
}
pub struct LabelIter<'a> {
    static_labels: slice::Iter<'a, Label>,
    dynamic_labels: slice::Iter<'a, Label>,
}
impl<'a> Iterator for LabelIter<'a> {
    type Item = &'a Label;
    fn next(&mut self) -> Option<Self::Item> {
        self.static_labels.next().or_else(
            || self.dynamic_labels.next(),
        )
    }
}

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
    pub fn clear(&mut self) {
        self.0.set(std::i64::MIN);
    }
    pub fn set(&mut self, timestamp: i64) {
        assert_ne!(timestamp, std::i64::MIN);
        self.0.set(timestamp);
    }
    pub fn set_time(&mut self, time: SystemTime) {
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
    pub fn set_now(&mut self) {
        self.set_time(SystemTime::now());
    }
}

#[cfg(target_pointer_width = "64")]
mod atomic {
    use std::marker::PhantomData;
    use std::mem;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // TODO: impl original Debug
    #[derive(Debug)]
    pub struct Atomic64<T> {
        value: AtomicUsize,
        _phantom: PhantomData<T>,
    }
    impl<T: Default + Copy> Atomic64<T> {
        pub fn new(value: T) -> Self {
            assert_eq!(mem::size_of::<T>(), mem::size_of::<usize>());
            Atomic64 {
                value: AtomicUsize::new(unsafe { mem::transmute_copy(&value) }),
                _phantom: PhantomData,
            }
        }
        pub fn get(&self) -> T {
            let value = self.value.load(Ordering::SeqCst);
            unsafe { mem::transmute_copy(&value) }
        }
        pub fn set(&self, value: T) {
            self.value.store(
                unsafe { mem::transmute_copy(&value) },
                Ordering::SeqCst,
            );
        }
    }
}
#[cfg(not(target_pointer_width = "64"))]
mod atomic {
    use std::sync::Mutex;

    #[derive(Debug)]
    pub struct Atomic64<T>(Mutex<T>);
    impl<T: Default + Copy> Atomic64<T> {
        pub fn new(value: T) -> Self {
            Atomic64(Mutex::new(value))
        }
        pub fn get(&self) -> T {
            if let Some(v) = self.0.lock().ok() {
                *v
            } else {
                T::default()
            }
        }
        pub fn set(&self, value: T) {
            if let Some(mut v) = self.0.lock().ok() {
                *v = value;
            }
        }
    }
}
impl<T: Default + Copy> atomic::Atomic64<T> {
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(T) -> T,
    {
        let v = self.get();
        self.set(f(v));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn atomic_f64_works() {
        let mut value = AtomicF64::new(0.0);
        assert_eq!(value.get(), 0.0);

        value.set(123456789.0);
        assert_eq!(value.get(), 123456789.0);

        value.update(|v| v + 1.0);
        assert_eq!(value.get(), 123456790.0);
    }
}
