use crate::mem::MemByteBuffer;
use core::{alloc::Layout, ptr::null_mut};

struct BigBufferVec {
    ptr: *mut MemByteBuffer,
    size: usize,
    capacity: usize,
    pages: usize,
}

impl BigBufferVec {
    const EMPTY: Self = Self {
        ptr: null_mut(),
        size: 0,
        capacity: 0,
        pages: 0,
    };

    unsafe fn alloc(&mut self, pages: usize) -> *mut u8 {
        unsafe {
            if self.size >= self.capacity {
                let new_pages = if self.pages == 0 {
                    self.ptr = super::paging::alloc_pages_cr3_kernel(1) as *mut _;
                    1
                } else {
                    let new_pages = self.pages << 1;
                    self.ptr = super::paging::realloc_pages_cr3(
                        self.ptr as *const u8,
                        self.pages,
                        new_pages,
                    ) as *mut _;
                    new_pages
                };

                self.pages = new_pages;
                self.capacity = (new_pages << 12) / size_of::<MemByteBuffer>();
            }

            let ptr = super::paging::alloc_pages_cr3_kernel(pages);
            let buffer = MemByteBuffer {
                start: ptr as usize,
                size: pages << 12,
            };

            *self.ptr.offset(self.size as isize) = buffer;
            self.size += 1;

            ptr
        }
    }

    unsafe fn dealloc(&mut self, start: *const u8, pages: usize) {
        unsafe {
            for i in 0..self.size {
                let entry = *self.ptr.offset(i as isize);

                if entry.start != start as usize {
                    continue;
                }

                assert!(entry.size == pages << 12);
                super::paging::free::ptr_pages_cr3(start, pages);
                self.size -= 1;
                *self.ptr.offset(i as isize) = *self.ptr.offset(self.size as isize);
            }
        }

        panic!("Attempted to free ptr that was not allocated");
    }
}

/// S is number of entries per page
struct ChunkBufferVec<const S: usize> {
    ptr: *mut BufferEntry<S>,
    size: usize,
    capacity: usize,
    pages: usize,
}

impl<const S: usize> ChunkBufferVec<S> {
    const EMPTY: Self = Self {
        ptr: null_mut(),
        size: 0,
        capacity: 0,
        pages: 0,
    };

    unsafe fn alloc(&mut self) -> *mut u8 {
        unsafe {
            for i in 0..self.size {
                if let Some(ptr) = (*self.ptr.offset(i as isize)).alloc() {
                    return ptr;
                }
            }

            if self.size >= self.capacity {
                let new_pages = if self.pages == 0 {
                    self.ptr = super::paging::alloc_pages_cr3_kernel(1) as *mut _;
                    1
                } else {
                    let new_pages = self.pages << 1;
                    self.ptr = super::paging::realloc_pages_cr3(
                        self.ptr as *const u8,
                        self.pages,
                        new_pages,
                    ) as *mut _;
                    new_pages
                };

                self.pages = new_pages;
                self.capacity = (new_pages << 12) / size_of::<BufferEntry<0>>();
            }

            let (new_entry, ptr) = BufferEntry::new();
            *self.ptr.offset(self.size as isize) = new_entry;
            self.size += 1;
            ptr
        }
    }

    unsafe fn dealloc(&mut self, ptr: *const u8) {
        unsafe {
            for i in 0..self.size {
                if (*self.ptr.offset(i as isize)).dealloc(ptr) {
                    return;
                }
            }

            panic!("Attempted to free ptr that was not allocated");
        }
    }
}

#[derive(Clone, Copy)]
/// S is number of entries per page
struct BufferEntry<const S: usize> {
    ptr: *mut u8,
    usage: u64,
}

impl<const S: usize> BufferEntry<S> {
    fn alloc(&mut self) -> Option<*mut u8> {
        unsafe {
            if self.usage == (1u64 << S).wrapping_sub(1) {
                return None;
            }

            for i in 0..S {
                let mask = 1 << i;
                if self.usage & mask == 0 {
                    self.usage |= mask;
                    return Some(self.ptr.offset((i * (4096 / S)) as isize));
                }
            }

            None
        }
    }

    fn dealloc(&mut self, ptr: *const u8) -> bool {
        let diff = (self.ptr as usize) - (ptr as usize);
        if diff >= 4096 {
            return false;
        }

        let i = diff / (4096 / S);
        debug_assert!(i < S);
        debug_assert!((self.usage >> i) & 1 == 1);
        self.usage &= !(1 << i);
        true
    }

    fn new() -> (Self, *mut u8) {
        unsafe {
            let ptr = super::paging::alloc_pages_cr3_kernel(1);
            let this = Self { ptr, usage: 1 };
            (this, ptr)
        }
    }
}

mod buffers {
    use crate::mem::alloc::{BigBufferVec, ChunkBufferVec};

    pub static mut BUFFER_64: ChunkBufferVec<64> = ChunkBufferVec::EMPTY;
    pub static mut BUFFER_128: ChunkBufferVec<32> = ChunkBufferVec::EMPTY;
    pub static mut BUFFER_256: ChunkBufferVec<16> = ChunkBufferVec::EMPTY;
    pub static mut BUFFER_512: ChunkBufferVec<8> = ChunkBufferVec::EMPTY;
    pub static mut BUFFER_1024: ChunkBufferVec<4> = ChunkBufferVec::EMPTY;
    pub static mut BUFFER_2048: ChunkBufferVec<2> = ChunkBufferVec::EMPTY;
    pub static mut BUFFER_BIG: BigBufferVec = BigBufferVec::EMPTY;
}

#[inline(always)]
pub unsafe fn alloc_t<T>() -> *mut T {
    unsafe { alloc(Layout::new::<T>()) as *mut T }
}

pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    #[allow(static_mut_refs)]
    unsafe {
        match layout.size() {
            0..=64 => buffers::BUFFER_64.alloc(),
            65..=128 => buffers::BUFFER_128.alloc(),
            129..=256 => buffers::BUFFER_256.alloc(),
            257..=512 => buffers::BUFFER_512.alloc(),
            513..=1024 => buffers::BUFFER_1024.alloc(),
            1025..=2048 => buffers::BUFFER_2048.alloc(),
            2049.. => buffers::BUFFER_BIG.alloc((layout.size() + 0xFFF) >> 12),
        }
    }
}

pub unsafe fn realloc_t<T, G>(ptr: *const T) -> *mut G {
    unsafe {
        let new_ptr = alloc_t();
        core::ptr::copy(
            ptr as *const u8,
            new_ptr as *mut u8,
            size_of::<T>().min(size_of::<G>()),
        );

        dealloc_t(ptr);
        new_ptr
    }
}

pub unsafe fn realloc(ptr: *const u8, old: Layout, new_size: usize) -> *mut u8 {
    unsafe {
        let new = Layout::from_size_align(new_size, old.align()).expect("Invalid layout");
        let new_ptr = alloc(new);
        core::ptr::copy(ptr, new_ptr, old.size().min(new_size));
        dealloc(ptr, old);
        new_ptr
    }
}

#[inline(always)]
pub unsafe fn dealloc_t<T>(ptr: *const T) {
    unsafe {
        dealloc(ptr as *const u8, Layout::new::<T>());
    }
}

pub unsafe fn dealloc(ptr: *const u8, layout: Layout) {
    #[allow(static_mut_refs)]
    unsafe {
        match layout.size() {
            0..=64 => buffers::BUFFER_64.dealloc(ptr),
            65..=128 => buffers::BUFFER_128.dealloc(ptr),
            129..=256 => buffers::BUFFER_256.dealloc(ptr),
            257..=512 => buffers::BUFFER_512.dealloc(ptr),
            513..=1024 => buffers::BUFFER_1024.dealloc(ptr),
            1025..=2048 => buffers::BUFFER_2048.dealloc(ptr),
            2049.. => buffers::BUFFER_BIG.dealloc(ptr, (layout.size() + 0xFFF) >> 12),
        }
    }
}
