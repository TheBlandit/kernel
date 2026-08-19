use super::{
    LIN_ADDR_MASK, MemPageBuffer, PAGING_TYPE, PHY_PAGE_MASK, PagingType, TO_HIGH_MASK, level4,
};

#[inline]
pub unsafe fn ptr_pages_cr3(ptr: *const u8, pages: usize) {
    unsafe { ptr_pages(ptr, pages, crate::read_reg!("cr3")) }
}

#[inline]
pub unsafe fn ptr_pages(ptr: *const u8, pages: usize, cr3: usize) {
    unsafe {
        let start = ((ptr as usize) & LIN_ADDR_MASK) >> 12;
        pages_buffer(MemPageBuffer { start, pages }, cr3);
    }
}

#[inline]
pub unsafe fn pages_buffer_cr3(buffer: MemPageBuffer) {
    unsafe { pages_buffer(buffer, crate::read_reg!("cr3")) }
}

pub unsafe fn pages_buffer(buffer: MemPageBuffer, cr3: usize) {
    unsafe {
        let cr3 = ((cr3 & PHY_PAGE_MASK) | TO_HIGH_MASK) as *mut usize;

        match PAGING_TYPE {
            PagingType::Level4 => {
                for page in buffer.start..(buffer.start + buffer.pages) {
                    level4::free_page(page, cr3);
                }
            }
            PagingType::Level5 => todo!(),
        }
    }
}
