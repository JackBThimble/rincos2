use core::cell::UnsafeCell;

pub struct SyncUnsafeCell<T> {
    inner: UnsafeCell<T>,
}

impl<T> SyncUnsafeCell<T> {
    pub const fn new(val: T) -> Self {
        Self {
            inner: UnsafeCell::new(val),
        }
    }

    #[inline(always)]
    pub fn get(&self) -> *mut T {
        self.inner.get()
    }
}

unsafe impl<T> Sync for SyncUnsafeCell<T> {}
