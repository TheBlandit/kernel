//! This file contains safe memory types

use core::ops::{Deref, DerefMut};

use super::alloc::{alloc_t, dealloc_t};

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

pub struct Rc<T> {
    ptr: *mut RcInner<T>,
}

impl<T> !Send for Rc<T> {}
impl<T> !Sync for Rc<T> {}

impl<T> Rc<T> {
    pub fn new(value: T) -> Self {
        unsafe {
            let inner = RcInner { value, count: 0 };
            let ptr = alloc_t::<RcInner<T>>();
            ptr.write(inner);

            Self { ptr }
        }
    }
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        unsafe {
            if let Some(count) = (*self.ptr).count.checked_add(1) {
                (*self.ptr).count = count;
            } else {
                panic!("Reference count overflow");
            }
        }

        Self { ptr: self.ptr }
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        unsafe {
            if let Some(count) = (*self.ptr).count.checked_sub(1) {
                (*self.ptr).count = count;
            } else {
                drop(self.ptr.read());
                dealloc_t(self.ptr);
            }
        }
    }
}

impl<T> Deref for Rc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &(*self.ptr).value }
    }
}

struct RcInner<T> {
    value: T,
    /// True count - 1
    count: usize,
}
