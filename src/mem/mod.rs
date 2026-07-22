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
    fn into_byte_buffer(&self) -> MemByteBuffer {
        // TODO: make canonical
        MemByteBuffer {
            start: self.start << 12,
            size: self.pages << 12,
        }
    }
}
