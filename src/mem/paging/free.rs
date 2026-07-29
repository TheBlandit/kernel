#[inline]
pub unsafe fn ptr_pages_cr3(ptr: *const u8, pages: usize) {
    unsafe { ptr_pages(ptr, pages, crate::read_reg!("cr3")) }
}

#[inline]
pub unsafe fn ptr_pages(ptr: *const u8, pages: usize, cr3: usize) {
    unsafe {
        let start = ((ptr as usize) & super::LIN_ADDR_MASK) >> 12;
        pages_buffer(super::MemPageBuffer { start, pages }, cr3);
    }
}

#[inline]
pub unsafe fn pages_buffer_cr3(buffer: super::MemPageBuffer) {
    unsafe { pages_buffer(buffer, crate::read_reg!("cr3")) }
}

pub unsafe fn pages_buffer(buffer: super::MemPageBuffer, cr3: usize) {
    unsafe {
        let cr3 = ((cr3 & super::PHY_PAGE_MASK) | super::TO_HIGH_MASK) as *mut usize;

        match super::PAGING_TYPE {
            super::PagingType::Level4 => {
                for page in buffer.start..(buffer.start + buffer.pages) {
                    super::level4::free_page(page, cr3);
                }
            }
            super::PagingType::Level5 => unimplemented!(),
        }
    }
}
