pub use self::atomic::Atomic64;

pub type AtomicF64 = atomic::Atomic64<f64>;
pub type AtomicU64 = atomic::Atomic64<u64>;

// TODO: rename
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
    impl Atomic64<u64> {
        pub fn inc(&self) {
            self.value.fetch_add(1, Ordering::SeqCst);
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
    impl Atomic64<u64> {
        pub fn inc(&self) {
            self.value.update(|v| *v + 1);
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
        let value = AtomicF64::new(0.0);
        assert_eq!(value.get(), 0.0);

        value.set(123456789.0);
        assert_eq!(value.get(), 123456789.0);

        value.update(|v| v + 1.0);
        assert_eq!(value.get(), 123456790.0);
    }
}
