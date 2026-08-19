use core::num::NonZero;

use super::{PHY_ADDR_BITS, TOTAL_PAGES};
use crate::mem::{
    MemPageBuffer,
    paging::{PAGING_TYPE, PHY_PAGE_MASK, PagingType, TO_HIGH_MASK, init::UEFIMemData, level4},
    wrappers::Vec,
};

static mut BUFFERS: Vec<MemPageBuffer> = Vec::new();

pub unsafe fn init(data: UEFIMemData) {
    unsafe {
        #[allow(static_mut_refs)]
        let vec = &mut BUFFERS;
        vec.push(MemPageBuffer::new(0, TOTAL_PAGES));

        data.for_each(|desc| {
            let phy_page = (desc.physical_start >> 12) as usize;
            let pages = desc.number_of_pages as usize;

            if (phy_page + pages) <= TOTAL_PAGES {
                return;
            }

            match vec.binary_search_by(|x| x.start.cmp(&phy_page)) {
                Ok(i) => {
                    let item = vec.get_unchecked_mut(i);
                    item.pages = item.pages.max(pages);
                }
                Err(i) => vec.insert(MemPageBuffer::new(phy_page, pages), i),
            }
        });

        loop {
            let mut br = true;

            for i in (0..(vec.len() - 1)).rev() {
                let second = *vec.get_unchecked(i + 1);
                let first = vec.get_unchecked_mut(i);

                if first.exclusive_end() >= second.start {
                    first.pages = first.pages.max(second.start + second.pages - first.start);
                    _ = vec.remove(i + 1);
                    br = false;
                }
            }

            if br {
                break;
            }
        }
    }
}

/// Returns physical address
#[must_use]
unsafe fn alloc_phy(pages: NonZero<usize>, phy_addr: Option<usize>) -> Option<usize> {
    unsafe {
        let pages = pages.get();
        #[allow(static_mut_refs)]
        let vec = &mut BUFFERS;

        if let Some(phy_addr) = phy_addr {
            assert!(phy_addr & 0xFFF == 0);
            let phy_page = phy_addr >> 12;

            if phy_page + pages >= 1 << (PHY_ADDR_BITS - 12) {
                return None;
            }

            let Err(start_index) = vec.binary_search_by(|x| x.inclusive_end().cmp(&phy_page))
            else {
                return None;
            };

            let end_page = phy_page + pages - 1;
            let Err(end_index) = vec.binary_search_by(|x| x.start.cmp(&end_page)) else {
                return None;
            };

            if start_index != end_index {
                return None;
            }

            vec.insert(MemPageBuffer::new(phy_page, pages), start_index);

            Some(phy_addr)
        } else {
            let mut prev_end = 0;

            for (i, buffer) in vec.iter().enumerate() {
                if prev_end + pages <= buffer.start {
                    vec.insert(MemPageBuffer::new(prev_end, pages), i);
                    return Some(prev_end << 12);
                } else {
                    prev_end = buffer.start + buffer.pages;
                }
            }

            if prev_end + pages >= 1 << (PHY_ADDR_BITS - 12) {
                return None;
            }

            vec.push(MemPageBuffer::new(prev_end, pages));

            Some(prev_end << 12)
        }
    }
}

/// Returns number of pages
#[must_use]
unsafe fn free_phy(phy_addr: usize) -> Option<usize> {
    #[allow(static_mut_refs)]
    unsafe {
        let vec = &mut BUFFERS;
        let index = vec.binary_search_by(|x| x.start.cmp(&phy_addr)).ok()?;
        Some(vec.remove(index).pages)
    }
}

/// Returns Option<(linear_address, physical_address)>
#[must_use]
pub unsafe fn alloc(
    pages: NonZero<usize>,
    kernel: bool,
    cr3: usize,
    phy_addr: Option<usize>,
) -> Option<(usize, usize)> {
    unsafe {
        let cr3 = ((cr3 & PHY_PAGE_MASK) | TO_HIGH_MASK) as *mut usize;
        let phy_addr = alloc_phy(pages, phy_addr)?;

        match PAGING_TYPE {
            PagingType::Level4 => {
                let lin_start = level4::get_lin_hole(pages.get(), cr3, kernel);
                let entry_mask = 3 | if kernel { 0 } else { 4 };

                for (i, page) in (lin_start..(lin_start + pages.get()))
                    .into_iter()
                    .enumerate()
                {
                    level4::allocate_page(page, phy_addr + (i << 12), cr3, entry_mask);
                }

                // Convert to pointer and make canonical
                let ptr = (((lin_start as isize) << 28) >> 16) as usize;
                Some((ptr, phy_addr))
            }
            PagingType::Level5 => todo!(),
        }
    }
}

pub unsafe fn free(lin_addr: usize, phy_addr: usize, cr3: usize) {
    unsafe {
        let cr3 = ((cr3 & PHY_PAGE_MASK) | TO_HIGH_MASK) as *mut usize;
        let pages = free_phy(phy_addr).expect("Nothing to free");

        match PAGING_TYPE {
            PagingType::Level4 => {
                for page in lin_addr..(lin_addr + pages) {
                    _ = level4::remove_page(page, cr3);
                }
            }
            PagingType::Level5 => todo!(),
        }
    }
}
