pub use self::atomic64::Atomic64;

pub type AtomicF64 = atomic64::Atomic64<f64>;
pub type AtomicU64 = atomic64::Atomic64<u64>;
pub type AtomicI64 = atomic64::Atomic64<i64>;

#[cfg(target_pointer_width = "64")]
mod atomic64 {
    use std::marker::PhantomData;
    use std::mem;
    use std::sync::atomic::{AtomicUsize, Ordering};

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

        #[inline]
        pub fn get(&self) -> T {
            let value = self.value.load(Ordering::SeqCst);
            unsafe { mem::transmute_copy(&value) }
        }

        #[inline]
        pub fn set(&self, value: T) {
            self.value
                .store(unsafe { mem::transmute_copy(&value) }, Ordering::SeqCst);
        }

        #[inline]
        pub fn update<F>(&self, f: F)
        where
            F: Fn(T) -> T,
        {
            loop {
                let old = self.get();
                let new = f(old);
                unsafe {
                    let old = mem::transmute_copy(&old);
                    let new = mem::transmute_copy(&new);
                    if self.value.compare_and_swap(old, new, Ordering::SeqCst) == old {
                        break;
                    }
                }
            }
        }
    }
    impl Atomic64<u64> {
        #[inline]
        pub fn inc(&self) {
            self.value.fetch_add(1, Ordering::SeqCst);
        }

        #[inline]
        pub fn add(&self, count: u64) {
            self.value.fetch_add(count as usize, Ordering::SeqCst);
        }
    }
}
#[cfg(not(target_pointer_width = "64"))]
mod atomic64 {
    use std::sync::Mutex;

    #[derive(Debug)]
    pub struct Atomic64<T>(Mutex<T>);
    impl<T: Default + Copy> Atomic64<T> {
        pub fn new(value: T) -> Self {
            Atomic64(Mutex::new(value))
        }

        #[inline]
        pub fn get(&self) -> T {
            if let Some(v) = self.0.lock().ok() {
                *v
            } else {
                T::default()
            }
        }

        #[inline]
        pub fn set(&self, value: T) {
            if let Some(mut v) = self.0.lock().ok() {
                *v = value;
            }
        }

        #[inline]
        pub fn update<F>(&self, f: F)
        where
            F: Fn(T) -> T,
        {
            loop {
                if let Some(mut v) = self.0.lock().ok() {
                    *v = f(*v);
                    break;
                }
            }
        }
    }
    impl Atomic64<u64> {
        #[inline]
        pub fn inc(&self) {
            self.value.update(|v| *v + 1);
        }

        #[inline]
        pub fn add(&self, count: u64) {
            self.value.update(|v| *v + count);
        }
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
