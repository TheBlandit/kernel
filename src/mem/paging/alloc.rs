use super::{PAGING_TYPE, PHY_PAGE_MASK, PageUse, PagingType, TO_HIGH_MASK, level4, phy};

/// Allocates pages for the current paging structure for the kernel
#[inline]
#[must_use]
pub unsafe fn pages_cr3_kernel(pages: usize) -> *mut u8 {
    unsafe { alloc_pages(pages, true, crate::read_reg!("cr3")) }
}

#[must_use]
pub unsafe fn alloc_pages(pages: usize, kernel: bool, cr3: usize) -> *mut u8 {
    unsafe {
        let cr3 = ((cr3 & PHY_PAGE_MASK) | TO_HIGH_MASK) as *mut usize;

        match PAGING_TYPE {
            PagingType::Level4 => {
                let lin_start = level4::get_lin_hole(pages, cr3, kernel);
                let entry_mask = 3 | if kernel { 0 } else { 4 };

                for page in lin_start..(lin_start + pages) {
                    level4::allocate_page(
                        page,
                        phy::allocate_zeroed_page(PageUse::Used) << 12,
                        cr3,
                        entry_mask,
                    );
                }

                // Convert to pointer and make canonical
                ((((lin_start as isize) << 28) >> 16) as usize) as *mut u8
            }
            PagingType::Level5 => unimplemented!(),
        }
    }
}
