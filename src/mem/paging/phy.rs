//! Physical memory

use super::{PHY_ADDR_BITS, PHY_PAGE_USE, PageUse, TO_HIGH_MASK, TOTAL_PAGES};
use core::num::NonZero;

#[inline(always)]
unsafe fn zero_page(page_index: usize) {
    unsafe {
        let ptr = ((page_index << 12) | TO_HIGH_MASK) as *mut u64;

        for i in 0..512 {
            ptr.offset(i).write(0);
        }
    }
}

/// Returns physical address (physical page must exist)
#[must_use]
pub unsafe fn contiguous_alloc_pages(
    zeroed: bool,
    pages: NonZero<usize>,
    phy_addr: Option<usize>,
) -> Option<usize> {
    unsafe {
        if let Some(phy_addr) = phy_addr {
            let start_page = phy_addr >> 12;

            if start_page + pages.get() > TOTAL_PAGES {
                return None;
            }

            for i in 0..pages.get() {
                if *PHY_PAGE_USE.offset((start_page + i) as isize) != PageUse::Free {
                    return None;
                }
            }

            for i in 0..pages.get() {
                *PHY_PAGE_USE.offset((start_page + i) as isize) = PageUse::Used;
                if zeroed {
                    zero_page(start_page + i);
                }
            }

            return Some(start_page << 12);
        } else {
            let pages_sub_1 = pages.get() - 1;
            let mut start_page = 0;

            for i in 0..TOTAL_PAGES {
                if *PHY_PAGE_USE.offset(i as isize) == PageUse::Free {
                    if i - start_page == pages_sub_1 {
                        for i in 0..pages.get() {
                            *PHY_PAGE_USE.offset((start_page + i) as isize) = PageUse::Used;
                            if zeroed {
                                zero_page(start_page + i);
                            }
                        }

                        return Some(start_page << 12);
                    }
                } else {
                    start_page = i + 1;
                }
            }
        }

        None
    }
}

/// Returns page index
#[must_use]
pub unsafe fn allocate_page_index(zeroed: bool) -> Option<usize> {
    unsafe {
        static mut PHY_PAGE_SEARCH_INDEX: usize = 0;

        for i in (PHY_PAGE_SEARCH_INDEX..TOTAL_PAGES).chain(0..PHY_PAGE_SEARCH_INDEX) {
            if *PHY_PAGE_USE.offset(i as isize) == PageUse::Free {
                PHY_PAGE_SEARCH_INDEX = i;
                *PHY_PAGE_USE.offset(i as isize) = PageUse::Used;

                if zeroed {
                    zero_page(i);
                }

                return Some(i);
            }
        }

        None
    }
}

/// Returns physical address
#[inline(always)]
#[must_use]
pub unsafe fn allocate_page_addr(zeroed: bool) -> Option<usize> {
    unsafe { allocate_page_index(zeroed).map(|x| x << 12) }
}

pub unsafe fn free_page(page_index: usize) {
    unsafe {
        assert!(
            page_index < TOTAL_PAGES,
            "Attempted to free a non-existent physical page"
        );
        assert!(
            core::mem::replace(
                &mut *PHY_PAGE_USE.offset(page_index as isize),
                PageUse::Free
            ) != PageUse::Free,
            "Attempted to free a freed physical page"
        );
    }
}
