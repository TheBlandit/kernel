#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(negative_impls)]

mod int;
mod mem;
mod output;
mod utils;

use core::ptr::null_mut;

use r_efi::efi::{Handle, Status, SystemTable};

#[repr(u32)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
enum BootState {
    Start = 0,
    Output = 1,
}

static mut BOOT_STATE: BootState = BootState::Start;

#[unsafe(export_name = "efi_main")]
pub extern "C" fn main(handle: Handle, table: *mut SystemTable) -> Status {
    unsafe {
        status_panic!(
            ((*(*table).boot_services).set_watchdog_timer)(0, 0, 0, null_mut()),
            "Set watchdog failed"
        );

        // Init uefi screen
        {
            let con_out = (*table).con_out;

            status_panic!(
                ((*con_out).clear_screen)(con_out),
                "UEFI clear screen failed"
            );

            status_panic!(
                ((*con_out).enable_cursor)(con_out, r_efi::efi::Boolean::TRUE),
                "UEFI enable cursor failed"
            );
        }

        let video_buffer = output::init(table);

        BOOT_STATE = BootState::Output;

        mem::paging::pre_exit_init(table);

        // Exit boot
        {
            let mut pages = 8usize;
            let boot_services = (*table).boot_services;

            let mem_data = loop {
                let mut memory_map: *mut r_efi::efi::MemoryDescriptor = null_mut();

                status_panic!(
                    ((*boot_services).allocate_pages)(
                        r_efi::efi::ALLOCATE_ANY_PAGES,
                        r_efi::efi::LOADER_DATA,
                        pages,
                        &mut memory_map as *mut _ as *mut u64,
                    ),
                    "UEFI exit allocate pages failure"
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
                    let status = ((*boot_services).exit_boot_services)(handle, map_key);

                    if !status.is_error() {
                        crate::output::raw_println(b"UEFI exit success");
                        break mem::paging::UEFIMemData {
                            buffer_size: memory_map_size,
                            desc_size,
                            ptr: memory_map,
                        };
                    }
                }

                status_panic!(
                    ((*boot_services).free_pages)(memory_map as u64, pages),
                    "UEFI exit free pages failure"
                );

                pages = (memory_map_size + 0x1FFF) >> 12; // Round up to nearest page and add 1 more
            };

            mem::paging::post_exit_init(mem_data);
        }
    }
}

/// Called after relocation to high address space
pub extern "C" fn high_entry() -> ! {
    unsafe {
        mem::paging::alloc_pages_this_kernel(10);

        crate::output::raw_println(b"Allocated");

        int::init();

        loop {
            core::arch::asm!("hlt");
        }
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    if unsafe { BOOT_STATE >= BootState::Output } {
        let message = info.message();

        if let Some(message) = message.as_str() {
            output::raw_print(b"PANIC '");
            output::raw_print(message.as_bytes());
            output::raw_print(b"'");
        } else {
            output::raw_print(b"PANIC");
        }

        if let Some(location) = info.location() {
            output::raw_print(b" FROM FILE: '");
            let file = location.file();
            // TODO: Finish relocation so all symbols are updated hence this OR will not be needed
            unsafe {
                output::raw_print(core::slice::from_raw_parts(
                    (file.as_ptr() as usize | mem::paging::TO_HIGH_MASK) as *const u8,
                    file.len(),
                ));
            }
            output::raw_print(b"', LINE: '");
            output::num::u32(location.line());
            output::raw_print(b"', COLUMN: '");
            output::num::u32(location.column());
            output::raw_println(b"'");
        } else {
            output::raw_println(b" FROM UNKNOWN");
        }
    }

    loop {
        unsafe {
            core::arch::asm!("cli", "hlt");
        }
    }
}
