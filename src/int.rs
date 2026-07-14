use core::ffi::c_void;

use crate::utils::DescTablePtr;

const INT_KEYBOARD: u8 = 0x21;
const ENTRIES: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
struct Idte {
    offset0: u16,
    cs: u16,
    attributes: u16,
    offset16: u16,
    offset32: u32,
    rsvd: u32,
}

#[repr(C)]
struct IntFrame {
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

static_assertions::const_assert_eq!(size_of::<Idte>(), 16);

impl Idte {
    const fn new(offset: u64, cs: u16, attributes: u16) -> Self {
        Self {
            cs,
            attributes,
            rsvd: 0,
            offset0: offset as u16,
            offset16: (offset >> 16) as u16,
            offset32: (offset >> 32) as u32,
        }
    }

    fn get_offset(&self) -> u64 {
        ((self.offset32 as u64) << 32) | ((self.offset16 as u64) << 16) | (self.offset0 as u64)
    }
}

unsafe fn inb(port: u16) -> u8 {
    unsafe {
        let value;

        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );

        value
    }
}

unsafe fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

static mut IDT: [Idte; ENTRIES] = [Idte::new(0, 0, 0); ENTRIES];

static mut IDTR: DescTablePtr = DescTablePtr {
    limit: (ENTRIES * size_of::<Idte>() - 1) as u16,
    ptr: 0,
};

mod handlers {
    use super::IntFrame;

    pub extern "x86-interrupt" fn none(_frame: IntFrame) -> ! {
        panic!("Unhandled interrupt");
    }

    pub extern "x86-interrupt" fn none_error(_frame: IntFrame, _error_code: u64) -> ! {
        panic!("Unhandled interrupt (with error code)");
    }

    pub extern "x86-interrupt" fn keyboard(_frame: IntFrame) {
        unsafe {
            let scancode = super::inb(0x60);
            crate::output::input_scancode(scancode);
            super::outb(0x20, 0x20);
        }
    }
}

/// Called after relocate
pub unsafe fn init() {
    #[allow(static_mut_refs)]
    unsafe {
        let cs: u16;
        core::arch::asm!(
            "mov ax, cs",
            out("ax") cs,
            options(nomem, nostack, preserves_flags)
        );

        for i in 0..ENTRIES {
            let handler = if [8, 10, 11, 12, 13, 14, 17, 21].contains(&i) {
                handlers::none_error as *const c_void
            } else {
                handlers::none as *const c_void
            } as usize as u64;

            IDT[i] = Idte::new(handler, cs, 0b1000111100000000);
        }

        IDT[INT_KEYBOARD as usize] = Idte::new(
            handlers::keyboard as *const c_void as usize as u64,
            cs,
            0b1000111100000000,
        );

        IDTR.ptr = IDT.as_ptr() as *const c_void as usize;

        core::arch::asm!(
            "lidt [{}]",
            "sti",
            in(reg) &IDTR as *const DescTablePtr as usize,
            options(nostack, nomem)
        );

        // Enable PIC
        outb(0x20, 0x11); // Initialize the command port
        outb(0x21, 0x20); // Set vector offset (IRQ0-IRQ7)
        outb(0x21, 0x04); // Set cascading (IRQ2)
        outb(0x21, 0x01); // Set 8086 mode
        outb(0x21, 0xFF); // Mask all interrupts initially

        outb(0x21, inb(0x21) & 0xFD); // Unmask IRQ1 (keyboard)
    }
}
