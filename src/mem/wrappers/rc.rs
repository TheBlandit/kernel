use super::prelude::*;

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
