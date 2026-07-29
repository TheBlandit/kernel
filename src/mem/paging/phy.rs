//! Physical memory

use super::{PHY_PAGE_USE, PageUse, TO_HIGH_MASK, TOTAL_PAGES};

pub unsafe fn allocate_page(page_use: PageUse) -> usize {
    unsafe {
        static mut PHY_PAGE_SEARCH_INDEX: usize = 0;

        for i in (PHY_PAGE_SEARCH_INDEX..TOTAL_PAGES).chain(0..PHY_PAGE_SEARCH_INDEX) {
            if *PHY_PAGE_USE.offset(i as isize) == PageUse::Free {
                PHY_PAGE_SEARCH_INDEX = i;
                *PHY_PAGE_USE.offset(i as isize) = page_use;
                return i;
            }
        }

        panic!("No free physical pages to allocate");
    }
}

pub unsafe fn allocate_zeroed_page(page_use: PageUse) -> usize {
    unsafe {
        let page = allocate_page(page_use);
        let ptr = ((page << 12) | TO_HIGH_MASK) as *mut u64;

        for i in 0..512 {
            ptr.offset(i).write(0);
        }

        page
    }
}

/// Non-cannonical
#[inline]
pub unsafe fn allocate_zeroed_addr(page_use: PageUse) -> usize {
    unsafe { allocate_zeroed_page(page_use) << 12 }
}

pub unsafe fn free_page(page: usize) {
    unsafe {
        assert!(
            page < TOTAL_PAGES,
            "Attempted to free a non-existent physical page"
        );
        assert!(
            core::mem::replace(&mut *PHY_PAGE_USE.offset(page as isize), PageUse::Free)
                != PageUse::Free,
            "Attempted to free a freed physical page"
        );
    }
}
