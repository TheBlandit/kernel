#[cfg(target_arch = "x86_64")]
mod level4;
mod phy;

pub mod alloc;
pub mod free;
pub mod realloc;

use crate::{mem::MemPageBuffer, utils::DescTablePtr};
use core::ptr::null_mut;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum PageUse {
    Free = 0,
    Used = 1,
    Null = 2,
    Invalid = 3,
    Unknown = 4,
    Loader = 5,
}

enum PagingType {
    #[cfg(target_arch = "x86")]
    /// 32 Bit
    Level2,
    #[cfg(target_arch = "x86")]
    /// PAE
    Level3,
    #[cfg(target_arch = "x86_64")]
    Level4,
    #[cfg(target_arch = "x86_64")]
    Level5,
}

static mut TOTAL_PAGES: usize = 0;
static mut PAGING_TYPE: PagingType = PagingType::Level4;
static mut PHY_PAGE_USE: *mut PageUse = null_mut();
/// Only valid after high_jump
static mut GDTR: DescTablePtr = DescTablePtr { ptr: 0, limit: 0 };
/// 1s in range (M,12]
static mut PHY_PAGE_MASK: usize = 0;
/// 1s in range (M,0]
static mut LIN_ADDR_MASK: usize = 0;
/// 1s in the range (63,M-1]
pub static mut TO_HIGH_MASK: usize = 0;

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
                r_efi::efi::CONVENTIONAL_MEMORY | r_efi::efi::PERSISTENT_MEMORY => PageUse::Free,

                r_efi::efi::LOADER_CODE | r_efi::efi::LOADER_DATA => PageUse::Used,

                r_efi::efi::RESERVED_MEMORY_TYPE
                | r_efi::efi::UNUSABLE_MEMORY
                | r_efi::efi::UNACCEPTED_MEMORY_TYPE
                | r_efi::efi::ACPI_RECLAIM_MEMORY
                | r_efi::efi::ACPI_MEMORY_NVS
                | r_efi::efi::MEMORY_MAPPED_IO
                | r_efi::efi::MEMORY_MAPPED_IO_PORT_SPACE
                | r_efi::efi::PAL_CODE => PageUse::Invalid,

                r_efi::efi::BOOT_SERVICES_CODE
                | r_efi::efi::BOOT_SERVICES_DATA
                | r_efi::efi::RUNTIME_SERVICES_CODE
                | r_efi::efi::RUNTIME_SERVICES_DATA => PageUse::Loader,

                _ => PageUse::Unknown,
            };

            let start = (desc.physical_start >> 12) as usize;

            for i in start..(start + desc.number_of_pages as usize).min(TOTAL_PAGES) {
                *PHY_PAGE_USE.offset(i as isize) = usage;
            }
        }

        *PHY_PAGE_USE = PageUse::Null;

        let cr4: usize = crate::read_reg!("cr4");

        PAGING_TYPE = if (cr4 >> 12) & 1 == 0 {
            PagingType::Level4
        } else {
            PagingType::Level5
        };

        let cr3: usize = crate::read_reg!("cr3");

        let cr3 = match PAGING_TYPE {
            PagingType::Level4 => level4::clone_uefi(cr3 as *const usize),
            PagingType::Level5 => {
                unimplemented!();
            }
        };

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
}

const GDT: &'static [u64; 5] = &[
    // Null
    0x0000_0000_0000_0000,
    // Kernel Code
    0x00AF_9A00_0000_FFFF,
    // Kernel Data
    0x00CF_9200_0000_FFFF,
    // User Code
    0x00AF_FA00_0000_FFFF,
    // User Data
    0x00CF_F200_0000_FFFF,
];

/// Called when relocating to a high address space
unsafe extern "C" fn high_jump() -> ! {
    #[allow(static_mut_refs)]
    unsafe {
        crate::output::raw_println(b"Post high address jump");

        crate::output::CONFIG.buffer =
            (crate::output::CONFIG.buffer as usize | TO_HIGH_MASK) as *mut u32;

        PHY_PAGE_USE = ((PHY_PAGE_USE as usize) | TO_HIGH_MASK) as *mut PageUse;

        GDTR.ptr = GDT as *const _ as usize;
        GDTR.limit = (GDT.len() * size_of::<u64>() - 1) as u16;

        core::arch::asm!(
            "lgdt [{gdt}]",

            "mov {tmp:x}, 16",
            "mov ds, {tmp:x}",
            "mov es, {tmp:x}",
            "mov ss, {tmp:x}",
            "mov gs, {tmp:x}",
            "mov fs, {tmp:x}",

            "push 8",
            "lea {tmp}, [2f + rip]",
            "push {tmp}",
            "retfq",
            "2:",

            gdt = in(reg) &GDTR,
            tmp = out(reg) _,
            options(preserves_flags),
        );

        crate::output::raw_println(b"Relocated output buffer and GDT");

        let cr3: usize = crate::read_reg!("cr3");
        let ptr = cr3 as *mut usize;

        for i in 0..256 {
            *ptr.offset(i) = 0;
        }

        crate::load_reg!("cr3", cr3);

        let mut total = 0;
        for page in 0..TOTAL_PAGES {
            if *PHY_PAGE_USE.offset(page as isize) == PageUse::Loader {
                total += 1;
                *PHY_PAGE_USE.offset(page as isize) = PageUse::Free;
            }
        }
        crate::output::print_num(total);
        crate::output::raw_println(b" pages freed");

        crate::high_entry();
    }
}

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
