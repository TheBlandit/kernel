use super::{PHY_PAGE_MASK, PageUse, TO_HIGH_MASK, phy};
use crate::mem::MemPageBuffer;

/// Returns physical address of page removed (but does not free the physical page)
#[must_use]
pub unsafe fn remove_page(page_index: usize, pml4: *mut usize) -> usize {
    unsafe {
        const MASK: usize = 0x1FF;
        let pml4i = (page_index >> 27) & MASK;
        let pdpti = (page_index >> 18) & MASK;
        let pdi = (page_index >> 9) & MASK;
        let pti = page_index & MASK;

        let pml4e = *pml4.offset(pml4i as isize);

        if pml4e & 1 == 0 {
            panic!("Attempted to free a linear address that wasn't allocated");
        }

        let pdpt = ((pml4e & PHY_PAGE_MASK) | TO_HIGH_MASK) as *mut usize;
        let pdpte = *pdpt.offset(pdpti as isize);

        if pdpte & 1 == 0 {
            panic!("Attempted to free a linear address that wasn't allocated");
        }

        if pdpte & (1 << 7) == 1 {
            panic!("Attempted to free a linear address that was allocated by a 1GB page");
        }

        let pd = ((pdpte & PHY_PAGE_MASK) | TO_HIGH_MASK) as *mut usize;
        let pde = *pd.offset(pdi as isize);

        if pde & 1 == 0 {
            panic!("Attempted to free a linear address that wasn't allocated");
        }

        if pde & (1 << 7) == 1 {
            panic!("Attempted to free a linear address that was allocated by a 2MB page");
        }

        let pt = ((pde & PHY_PAGE_MASK) | TO_HIGH_MASK) as *mut usize;
        let pte = *pt.offset(pti as isize);

        if pte & 1 == 0 {
            panic!("Attempted to free a linear address that wasn't allocated");
        } else {
            let page = pte & PHY_PAGE_MASK;
            *pt.offset(pti as isize) = 0;
            // TODO: if not current CR3, INVLPG page
            page
        }
    }
}

pub unsafe fn free_page(page_index: usize, pml4: *mut usize) {
    unsafe {
        phy::free_page(remove_page(page_index, pml4) >> 12);
    }
}

pub unsafe fn allocate_page(
    linear_page_index: usize,
    physical_page_addr: usize,
    pml4: *mut usize,
    entry_mask: usize,
) {
    unsafe {
        let entry = |ptr: *mut usize| {
            let addr = phy::allocate_page_addr(true).unwrap();
            *ptr = addr | entry_mask;
            addr | TO_HIGH_MASK
        };

        const MASK: usize = 0x1FF;
        let pml4i = (linear_page_index >> 27) & MASK;
        let pdpti = (linear_page_index >> 18) & MASK;
        let pdi = (linear_page_index >> 9) & MASK;
        let pti = linear_page_index & MASK;

        let pml4e = *pml4.offset(pml4i as isize);

        let pdpt = if pml4e & 1 == 1 {
            (pml4e & PHY_PAGE_MASK) | TO_HIGH_MASK
        } else {
            entry(pml4.offset(pml4i as isize))
        } as *mut usize;

        let pdpte = *pdpt.offset(pdpti as isize);

        let pd = if pdpte & 1 == 1 {
            if pdpte & (1 << 7) == 1 {
                panic!(
                    "Attempted to allocate a linear address that was already allocated by a 1GB page"
                );
            }

            (pdpte & PHY_PAGE_MASK) | TO_HIGH_MASK
        } else {
            entry(pdpt.offset(pdpti as isize))
        } as *mut usize;

        let pde = *pd.offset(pdi as isize);

        let pt = if pde & 1 == 1 {
            if pde & (1 << 7) == 1 {
                panic!(
                    "Attempted to allocate a linear address that was already allocated by a 2MB page"
                );
            }

            (pde & PHY_PAGE_MASK) | TO_HIGH_MASK
        } else {
            entry(pd.offset(pdi as isize))
        } as *mut usize;

        let pte = *pt.offset(pti as isize);

        if pte & 1 == 1 {
            panic!(
                "Attempted to allocate a linear address that was already allocated by a 4KB page"
            );
        } else {
            *pt.offset(pti as isize) = physical_page_addr | entry_mask;
        }
    }
}

/// Returns page at the start of the hole
pub unsafe fn get_lin_hole(pages: usize, pml4: *const usize, kernel: bool) -> usize {
    unsafe {
        let mut buffer: Option<MemPageBuffer> = None;

        macro_rules! not_present {
            ($size:ident, $start:expr) => {
                if let Some(buffer) = &mut buffer {
                    buffer.pages += $size;
                    if buffer.pages >= pages {
                        return buffer.start;
                    }
                } else {
                    if $size >= pages {
                        return $start;
                    } else {
                        buffer = Some(MemPageBuffer {
                            start: $start,
                            pages: $size,
                        });
                    }
                }
            };
        }

        for pml4i in if kernel { 256..512 } else { 0..256 } {
            const PML4_SHL: usize = 27;
            const PML4_SIZE: usize = 1 << PML4_SHL;

            let pml4e = *pml4.offset(pml4i as isize);

            if pml4e & 1 == 0 {
                not_present!(PML4_SIZE, pml4i << PML4_SHL);
            } else {
                let pdpt = ((pml4e & PHY_PAGE_MASK) | TO_HIGH_MASK) as *const usize;

                for pdpti in 0..512 {
                    const PDPT_SHL: usize = 18;
                    const PDPT_SIZE: usize = 1 << PDPT_SHL;

                    let pdpte = *pdpt.offset(pdpti as isize);

                    if pdpte & 1 == 0 {
                        not_present!(PDPT_SIZE, (pml4i << PML4_SHL) | (pdpti << PDPT_SHL));
                    } else if pdpte & (1 << 7) != 0 {
                        // 1 GB PAGE
                        buffer = None;
                    } else {
                        let pd = ((pdpte & PHY_PAGE_MASK) | TO_HIGH_MASK) as *const usize;

                        for pdi in 0..512 {
                            const PD_SHL: usize = 9;
                            const PD_SIZE: usize = 1 << PD_SHL;

                            let pde = *pd.offset(pdi as isize);

                            if pde & 1 == 0 {
                                not_present!(
                                    PD_SIZE,
                                    (pml4i << PML4_SHL) | (pdpti << PDPT_SHL) | (pdi << PD_SHL)
                                );
                            } else if pde & (1 << 7) != 0 {
                                // 2MB page
                                buffer = None;
                            } else {
                                let pt = ((pde & PHY_PAGE_MASK) | TO_HIGH_MASK) as *const usize;

                                for pti in 0..512 {
                                    const PT_SHL: usize = 0;
                                    const PT_SIZE: usize = 1 << PT_SHL;

                                    let pte = *pt.offset(pti as isize);

                                    if pte & 1 == 0 {
                                        not_present!(
                                            PT_SIZE,
                                            (pml4i << PML4_SHL)
                                                | (pdpti << PDPT_SHL)
                                                | (pdi << PD_SHL)
                                                | (pti << PT_SHL)
                                        )
                                    } else {
                                        buffer = None
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        panic!("Unable to find hole in linear address space");
    }
}

pub unsafe fn clone_uefi(opml4: *const usize) -> usize {
    unsafe {
        let npml4 = phy::allocate_page_addr(true).unwrap() as *mut usize;

        for pml4i in 0..256 {
            let opml4e = *opml4.offset(pml4i as isize);

            if opml4e & 1 != 0 {
                let npdpt = phy::allocate_page_addr(true).unwrap() as *mut usize;
                *npml4.offset(pml4i as isize) = (npdpt as usize) | 3;

                let opdpt = ((opml4e & PHY_PAGE_MASK) | TO_HIGH_MASK) as *const usize;

                for pdpti in 0..512 {
                    let opdpte = *opdpt.offset(pdpti as isize);

                    if opdpte & 1 != 0 {
                        if opdpte & (1 << 7) != 0 {
                            // 1 GB PAGE
                            *npdpt.offset(pdpti as isize) =
                                (opdpte & PHY_PAGE_MASK & !0x3FFF_FFFF) | 0b1000_0011;
                        } else {
                            let npd = phy::allocate_page_addr(true).unwrap() as *mut usize;
                            *npdpt.offset(pdpti as isize) = (npd as usize) | 3;

                            let opd = ((opdpte & PHY_PAGE_MASK) | TO_HIGH_MASK) as *const usize;

                            for pdi in 0..512 {
                                let opde = *opd.offset(pdi as isize);

                                if opde & 1 != 0 {
                                    if opde & (1 << 7) != 0 {
                                        // 2MB page
                                        *npd.offset(pdi as isize) =
                                            (opde & PHY_PAGE_MASK & !0x1F_FFFF) | 0b1000_0011;
                                    } else {
                                        let npt =
                                            phy::allocate_page_addr(true).unwrap() as *mut usize;
                                        *npd.offset(pdi as isize) = (npt as usize) | 3;

                                        let opt =
                                            ((opde & PHY_PAGE_MASK) | TO_HIGH_MASK) as *const usize;

                                        for pti in 0..512 {
                                            let opte = *opt.offset(pti as isize);

                                            if opte & 1 != 0 {
                                                // 4KB page
                                                *npt.offset(pti as isize) =
                                                    (opde & PHY_PAGE_MASK) | 3;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for i in 0..256 {
            *npml4.offset(i + 256) = *npml4.offset(i);
        }

        npml4 as usize
    }
}
