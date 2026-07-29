use super::{alloc, free};

pub unsafe fn pages_cr3(ptr: *const u8, old_pages: usize, new_pages: usize) -> *mut u8 {
    unsafe { pages(ptr, old_pages, new_pages, crate::read_reg!("cr3")) }
}

pub unsafe fn pages(ptr: *const u8, old_pages: usize, new_pages: usize, cr3: usize) -> *mut u8 {
    unsafe {
        // TODO: see if current allocation can be extended
        let kernel = ((ptr as usize) >> 63) & 1 == 1;
        let new_ptr = alloc::alloc_pages(new_pages, kernel, cr3);
        core::ptr::copy(ptr, new_ptr, old_pages.min(new_pages) << 12);
        free::ptr_pages_cr3(ptr, old_pages);
        new_ptr
    }
}
