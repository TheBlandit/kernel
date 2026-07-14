pub mod boxed;
pub mod paging;

pub struct MemByteBuffer {
    pub start: usize,
    pub size: usize,
}

pub struct MemPageBuffer {
    pub start: usize,
    pub pages: usize,
}

impl MemPageBuffer {
    fn into_byte_buffer(&self) -> MemByteBuffer {
        MemByteBuffer {
            start: self.start << 12,
            size: self.pages << 12,
        }
    }
}
