pub mod alloc;
pub mod paging;
pub mod wrappers;

#[derive(Clone, Copy)]
pub struct MemByteBuffer {
    pub start: usize,
    pub size: usize,
}

#[derive(Clone, Copy)]
pub struct MemPageBuffer {
    pub start: usize,
    pub pages: usize,
}

impl MemPageBuffer {
    #[inline(always)]
    pub fn new(start: usize, pages: usize) -> Self {
        Self { start, pages }
    }

    /// Last page in the buffer
    #[inline(always)]
    pub fn inclusive_end(&self) -> usize {
        self.exclusive_end() - 1
    }

    /// First page after the buffer
    #[inline(always)]
    pub fn exclusive_end(&self) -> usize {
        self.start + self.pages
    }
}
