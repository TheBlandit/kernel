use super::prelude::*;

pub struct Vec<T> {
    ptr: *mut T,
    capacity: usize,
    len: usize,
}

impl<T> Vec<T> {
    pub const fn new() -> Self {
        Self {
            ptr: null_mut(),
            capacity: 0,
            len: 0,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        if cap == 0 {
            Self::new()
        } else {
            let layout = Layout::from_size_align(size_of::<T>() * cap, align_of::<T>()).unwrap();

            Self {
                ptr: unsafe { alloc(layout) as _ },
                capacity: cap,
                len: 0,
            }
        }
    }

    pub fn push(&mut self, value: T) {
        unsafe {
            if self.len >= self.capacity {
                if self.capacity == 0 {
                    let layout =
                        Layout::from_size_align(size_of::<T>() << 2, align_of::<T>()).unwrap();
                    self.ptr = alloc(layout) as _;
                    self.capacity = 4;
                } else {
                    let new_cap = self.capacity << 1;
                    let old_layout =
                        Layout::from_size_align(size_of::<T>() * self.capacity, align_of::<T>())
                            .unwrap();

                    self.ptr =
                        realloc(self.ptr as *const u8, old_layout, size_of::<T>() * new_cap) as _;
                    self.capacity = new_cap;
                }
            }

            self.ptr.offset(self.len as isize).write(value);
            self.len += 1;
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(unsafe { self.ptr.offset(self.len as isize).read() })
        }
    }

    pub fn peek(&self) -> Option<&T> {
        if self.len == 0 {
            None
        } else {
            Some(unsafe { &*self.ptr.offset(self.len as isize - 1) })
        }
    }

    pub fn peek_mut(&self) -> Option<&mut T> {
        if self.len == 0 {
            None
        } else {
            Some(unsafe { &mut *self.ptr.offset(self.len as isize - 1) })
        }
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        if i < self.len {
            Some(unsafe { &*self.ptr.offset(i as isize) })
        } else {
            None
        }
    }

    pub fn get_mut(&self, i: usize) -> Option<&mut T> {
        if i < self.len {
            Some(unsafe { &mut *self.ptr.offset(i as isize) })
        } else {
            None
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T> Deref for Vec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> AsRef<[T]> for Vec<T> {
    fn as_ref(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T> AsMut<[T]> for Vec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe {
                drop(self.ptr.offset(i as isize).read());
            }
        }
    }
}
