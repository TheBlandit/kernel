use core::ptr::null_mut;

use crate::{mem::MemPageBuffer, utils::DescTablePtr};

macro_rules! read_reg {
    ($x:literal) => {
        {
            let reg;
            core::arch::asm!(
                concat!("mov {}, ", $x),
                out(reg) reg,
                options(nomem, preserves_flags, nostack)
            );
            reg
        }
    };
}

macro_rules! load_reg {
    ($x:literal, $y:expr) => {
        core::arch::asm!(
            concat!("mov ", $x, ", {}"),
            in(reg) $y,
            options(nomem, preserves_flags, nostack)
        )
    };
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum PageUse {
    Free = 0,
    Used = 1,
    Null = 2,
}

enum PagingType {
    Level4,
    Level5,
}

static mut TOTAL_PAGES: usize = 0;
static mut PAGING_TYPE: PagingType = PagingType::Level4;
static mut PHY_PAGE_USE: *mut PageUse = null_mut();

pub struct UEFIMemData {
    pub buffer_size: usize,
    pub desc_size: usize,
    pub ptr: *const r_efi::efi::MemoryDescriptor,
}

pub unsafe fn post_exit_init(data: UEFIMemData) -> ! {
    unsafe {
        // Masks
        {
            let eax = core::arch::x86_64::__cpuid(0x8000_0008).eax;
            let phy_addr_bits = eax & 0xFF;
            let lin_addr_bits = (eax >> 8) & 0xFF;
            PHY_PAGE_MASK = ((1 << phy_addr_bits) - 1) & !0xFFF;
            LIN_ADDR_MASK = (1 << lin_addr_bits) - 1;
        }

        for i in 0..(data.buffer_size / data.desc_size) {
            let desc = *data.ptr.byte_offset((i * data.desc_size) as isize);

            let usage = match desc.r#type {
                r_efi::efi::CONVENTIONAL_MEMORY => PageUse::Free,
                _ => PageUse::Used,
            };

            let start = (desc.physical_start >> 12) as usize;

            for i in start..(start + desc.number_of_pages as usize).min(TOTAL_PAGES) {
                *PHY_PAGE_USE.offset(i as isize) = usage;
            }
        }

        *PHY_PAGE_USE = PageUse::Null;

        let cr4: usize = read_reg!("cr4");

        PAGING_TYPE = if (cr4 >> 12) & 1 == 0 {
            PagingType::Level4
        } else {
            PagingType::Level5
        };

        match PAGING_TYPE {
            PagingType::Level4 => {
                let cr3 = phy::allocate_zeroed_page(PageUse::Used) << 12;
                let current_cr3: usize = read_reg!("cr3");

                for i in 0..256 {
                    *(cr3 as *mut usize).offset(i) = *(current_cr3 as *const usize).offset(i);
                }

                for i in 0..256 {
                    *(cr3 as *mut usize).offset(256 + i) = *(current_cr3 as *const usize).offset(i);
                }

                let eax = core::arch::x86_64::__cpuid(0x8000_0008).eax;
                let lin_addr_bits = ((eax >> 8) & 0xFF) as usize;
                assert_eq!(lin_addr_bits, 48, "Unsupported linear address size");
                TO_HIGH_MASK = !((1 << (lin_addr_bits - 1)) - 1);

                crate::output::raw_println(b"Pre high address jump");

                core::arch::asm!(
                    "or rbp, {mask}",
                    "or rsp, {mask}",
                    "mov cr3, {cr3}",
                    "or {entry}, {mask}",
                    "jmp {entry}",
                    cr3 = in(reg) cr3,
                    entry = in(reg) high_jump,
                    mask = in(reg) TO_HIGH_MASK,
                    options(noreturn),
                );
            }
            PagingType::Level5 => {
                unimplemented!();
            }
        }
    }
}

/// Called when relocating to a high address space
unsafe extern "C" fn high_jump() -> ! {
    #[allow(static_mut_refs)]
    unsafe {
        crate::output::raw_println(b"Post high address jump");

        crate::output::CONFIG.buffer =
            (crate::output::CONFIG.buffer as usize | TO_HIGH_MASK) as *mut u32;

        PHY_PAGE_USE = ((PHY_PAGE_USE as usize) | TO_HIGH_MASK) as *mut PageUse;

        core::arch::asm!(
            "sgdt [{}]",
            in(reg) &mut GDTR as *mut _ as usize,
        );

        GDTR.ptr |= TO_HIGH_MASK;

        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &mut GDTR as *mut _ as usize,
        );

        crate::output::raw_println(b"Relocated output buffer and GDT");

        let cr3: usize = read_reg!("cr3");
        let ptr = cr3 as *mut usize;

        for i in 0..256 {
            *ptr.offset(i) = 0;
        }

        load_reg!("cr3", cr3);

        crate::high_entry();
    }
}

/// Only valid after high_jump
static mut GDTR: DescTablePtr = DescTablePtr { ptr: 0, limit: 0 };

pub unsafe fn pre_exit_init(table: *mut r_efi::efi::SystemTable) {
    unsafe {
        let mut pages = 8usize;
        let boot_services = (*table).boot_services;

        let mem_data = loop {
            let mut memory_map: *mut r_efi::efi::MemoryDescriptor = null_mut();

            crate::status_panic!(
                ((*boot_services).allocate_pages)(
                    r_efi::efi::ALLOCATE_ANY_PAGES,
                    r_efi::efi::LOADER_DATA,
                    pages,
                    &mut memory_map as *mut _ as *mut u64,
                ),
                "UEFI memory pre-exit allocate pages failure"
            );

            let mut memory_map_size = pages << 12;

            let mut map_key = 0usize;
            let mut desc_size = 0usize;
            let mut desc_version = 0u32;

            let status = ((*boot_services).get_memory_map)(
                &mut memory_map_size as *mut usize,
                memory_map,
                &mut map_key,
                &mut desc_size,
                &mut desc_version,
            );

            if !status.is_error() {
                break UEFIMemData {
                    buffer_size: memory_map_size,
                    desc_size,
                    ptr: memory_map,
                };
            }

            crate::status_panic!(
                ((*boot_services).free_pages)(memory_map as u64, pages),
                "UEFI memory pre-exit free pages failure"
            );

            pages = (memory_map_size + 0x1FFF) >> 12; // Round up to nearest page and add 1 more
        };

        let mut mem_size = 0;

        for i in 0..(mem_data.buffer_size / mem_data.desc_size) {
            let desc = *mem_data.ptr.byte_offset((i * mem_data.desc_size) as isize);

            if desc.r#type != r_efi::efi::RESERVED_MEMORY_TYPE
                && desc.r#type != r_efi::efi::MEMORY_MAPPED_IO
            {
                mem_size = mem_size.max((desc.number_of_pages << 12) + desc.physical_start);
            }
        }

        debug_assert_eq!(mem_size, 512 << 20, "UEFI reporting incorrect memory size");

        TOTAL_PAGES = (mem_size >> 12) as usize;

        let mut ptr = null_mut();

        crate::status_panic!(
            ((*boot_services).allocate_pages)(
                r_efi::efi::ALLOCATE_ANY_PAGES,
                r_efi::efi::LOADER_DATA,
                (TOTAL_PAGES * size_of::<PageUse>() + 0xFFF) >> 12,
                &mut ptr as *mut _ as *mut u64,
            ),
            "UEFI memory pre-exit allocate pages failure"
        );

        PHY_PAGE_USE = ptr;

        for i in 0..TOTAL_PAGES {
            *PHY_PAGE_USE.offset(i as isize) = PageUse::Used;
        }
    }
}

/// Allocates pages for the current paging structure for the kernel
#[inline]
#[must_use]
pub unsafe fn alloc_pages_cr3_kernel(pages: usize) -> *mut u8 {
    unsafe { alloc_pages(pages, true, read_reg!("cr3")) }
}

#[must_use]
pub unsafe fn alloc_pages(pages: usize, kernel: bool, cr3: usize) -> *mut u8 {
    unsafe {
        let cr3 = ((cr3 & PHY_PAGE_MASK) | TO_HIGH_MASK) as *mut usize;

        match PAGING_TYPE {
            PagingType::Level4 => {
                let lin_start = level4_get_lin_hole(pages, cr3, kernel);
                let entry_mask = 3 | if kernel { 0 } else { 4 };

                for page in lin_start..(lin_start + pages) {
                    level4_allocate_page(
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

pub mod free {
    #[inline]
    pub unsafe fn ptr_pages_cr3(ptr: *const u8, pages: usize) {
        unsafe { ptr_pages(ptr, pages, read_reg!("cr3")) }
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
        unsafe { pages_buffer(buffer, read_reg!("cr3")) }
    }

    pub unsafe fn pages_buffer(buffer: super::MemPageBuffer, cr3: usize) {
        unsafe {
            let cr3 = ((cr3 & super::PHY_PAGE_MASK) | super::TO_HIGH_MASK) as *mut usize;

            match super::PAGING_TYPE {
                super::PagingType::Level4 => {
                    for page in buffer.start..(buffer.start + buffer.pages) {
                        super::level4_free_page(page, cr3);
                    }
                }
                super::PagingType::Level5 => unimplemented!(),
            }
        }
    }
}

/// 1s in range (M,12]
static mut PHY_PAGE_MASK: usize = 0;
static mut LIN_ADDR_MASK: usize = 0;
pub static mut TO_HIGH_MASK: usize = 0;

unsafe fn level4_free_page(page_index: usize, pml4: *mut usize) {
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
            let page = (pte & PHY_PAGE_MASK) >> 12;
            phy::free_page(page);
            *pt.offset(pti as isize) = 0;
        }

        // TODO: if not current CR3, INVLPG page
    }
}

unsafe fn level4_allocate_page(
    linear_page_index: usize,
    physical_page_addr: usize,
    pml4: *mut usize,
    entry_mask: usize,
) {
    unsafe {
        let entry = |ptr: *mut usize| {
            let addr = phy::allocate_zeroed_page(PageUse::Used) << 12;
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
unsafe fn level4_get_lin_hole(pages: usize, pml4: *const usize, kernel: bool) -> usize {
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

mod phy {
    //! Physical memory
    use crate::mem::paging::TO_HIGH_MASK;

    use super::{PHY_PAGE_USE, PageUse, TOTAL_PAGES};

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
}
