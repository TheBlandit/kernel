use super::prelude::*;

pub struct Box<T> {
    ptr: *mut T,
}

impl<T> Box<T> {
    pub fn new(value: T) -> Self {
        unsafe {
            let ptr = alloc_t::<T>();
            ptr.write(value);

            Self { ptr }
        }
    }

    pub fn take(this: Self) -> T {
        unsafe { this.ptr.read() }
    }

    pub fn leak(this: Self) -> &'static mut T {
        let ret = unsafe { &mut *this.ptr };
        core::mem::forget(this);
        ret
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            drop(self.ptr.read());
            dealloc_t(self.ptr)
        };
    }
}

impl<T> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}
