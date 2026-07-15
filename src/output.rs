use core::ptr::null_mut;
use r_efi::{efi::SystemTable, protocols::graphics_output::GraphicsPixelFormat};

use crate::{mem::MemByteBuffer, status_panic};

pub struct StaticData {
    pub buffer: *mut u32,
    size: usize,
    pixel_w: usize,
    pixel_h: usize,
    char_w: usize,
    char_h: usize,
    pitch: usize,
    format: GraphicsPixelFormat,
}

pub static mut CONFIG: StaticData = StaticData {
    buffer: null_mut(),
    size: 0,
    pixel_w: 0,
    pixel_h: 0,
    char_w: 0,
    char_h: 0,
    pitch: 0,
    format: 0,
};

static mut CURSOR: usize = 0;

const CHAR_H: usize = 16;
const CHAR_W: usize = 9;
const BACKGROUND: u32 = 0x00101010;

/// Returns video buffer
pub unsafe fn init(table: *mut SystemTable) -> MemByteBuffer {
    unsafe {
        let boot_services = (*table).boot_services;

        let mut handle_count = 0;
        let mut handle_buffer = null_mut();
        let mut guid = r_efi::protocols::graphics_output::PROTOCOL_GUID;

        // Locate GOP
        status_panic!(
            ((*boot_services).locate_handle_buffer)(
                r_efi::efi::BY_PROTOCOL,
                &mut guid,
                null_mut(),
                &mut handle_count,
                &mut handle_buffer,
            ),
            "UEFI locate GOP failed"
        );

        let mut gop: *mut r_efi::protocols::graphics_output::Protocol = null_mut();
        let mut guid = r_efi::protocols::graphics_output::PROTOCOL_GUID;

        status_panic!(
            ((*boot_services).handle_protocol)(
                *handle_buffer,
                &mut guid,
                &mut gop as *mut *mut r_efi::protocols::graphics_output::Protocol
                    as *mut *mut core::ffi::c_void,
            ),
            "UEFI handle GOP protocol failed"
        );

        // Set mode
        status_panic!(((*gop).set_mode)(gop, 0), "UEFI set GOP mode failed");

        let mode = (*gop).mode;
        let info = (*mode).info;

        let video_buffer = MemByteBuffer {
            start: (*mode).frame_buffer_base as usize,
            size: (*mode).frame_buffer_size,
        };

        {
            let width = (*info).horizontal_resolution as usize;
            let height = (*info).vertical_resolution as usize;
            let pitch = (*info).pixels_per_scan_line as usize;
            let buffer = video_buffer.start as *mut u32;

            CONFIG = StaticData {
                buffer,
                size: video_buffer.size,
                pixel_w: width,
                pixel_h: height,
                char_w: width / CHAR_W,
                char_h: height / CHAR_H,
                pitch,
                format: (*info).pixel_format,
            };

            for y in 0..(CONFIG.char_h * CHAR_H) {
                let yy = y * pitch;
                for x in 0..(CONFIG.char_w * CHAR_W) {
                    *buffer.offset((x + yy) as isize) = BACKGROUND;
                }
            }
        }

        video_buffer
    }
}

// Shift all lines up by 1
fn shift_up() {
    unsafe {
        let buffer = CONFIG.buffer;
        let pitch = CONFIG.pitch;

        for y in 0..(CHAR_H * (CONFIG.char_h - 1)) {
            let yy = y * pitch;
            for x in 0..(CHAR_W * CONFIG.char_w) {
                let pos = yy + x;
                *buffer.offset(pos as isize) = *buffer.offset((pos + pitch * CHAR_H) as isize);
            }
        }

        for y in (CHAR_H * (CONFIG.char_h - 1))..(CHAR_H * CONFIG.char_h) {
            let yy = y * pitch;
            for x in 0..(CHAR_W * CONFIG.char_w) {
                *buffer.offset((yy + x) as isize) = BACKGROUND;
            }
        }
    }
}

#[must_use = "If false, character was not printed"]
fn print_char(glyph: &[u8], x: usize, y: usize, colour: u32) -> bool {
    let base_x = x * CHAR_W;
    let base_y = y * CHAR_H;
    let pitch = unsafe { CONFIG.pitch };
    let buffer = unsafe { CONFIG.buffer };

    if x >= unsafe { CONFIG.char_w } && y >= unsafe { CONFIG.char_h } {
        return false;
    }

    for x in 0..CHAR_W {
        let col = x << 1;
        let col_data = (glyph[col] as u16) + ((glyph[col + 1] as u16) << 8);

        for y in 0..CHAR_H {
            let pixel = if (col_data >> y) & 1 == 1 {
                colour
            } else {
                BACKGROUND
            };

            unsafe {
                *buffer.offset((base_x + x + (base_y + y) * pitch) as isize) = pixel;
            }
        }
    }

    true
}

pub fn raw_print(string: &[u8]) {
    unsafe {
        let y = CONFIG.char_h - 1;

        for &char in string {
            if CURSOR >= CONFIG.char_w {
                CURSOR = 0;
                shift_up();
            }

            let start = (char as usize) * 18;
            assert!(print_char(&FONT_DATA[start..], CURSOR, y, u32::MAX));

            CURSOR += 1;
        }
    }
}

pub fn raw_println(string: &[u8]) {
    raw_print(string);
    unsafe { CURSOR = 0 };
    shift_up();
}

pub fn input_scancode(scancode: u8) {
    unsafe {
        static mut SHIFT: u8 = 0;

        if scancode < 0x80 {
            let scancode = scancode | SHIFT;
            let map = SCANCODE_MAP[scancode as usize];
            if map == 0 {
                if scancode == 0x2A {
                    SHIFT = 0x80;
                } else if scancode == 0x0E {
                    // Backspace
                    if CURSOR != 0 {
                        CURSOR -= 1;
                        assert!(print_char(
                            &FONT_DATA[((b' ' as usize) * 18)..],
                            CURSOR,
                            CONFIG.char_h - 1,
                            0xFFFFFFFF
                        ));
                    }
                } else if scancode == 0x1C {
                    shift_up();
                    CURSOR = 0;
                }
            } else {
                assert!(print_char(
                    &FONT_DATA[((map as u8 as usize) * 18)..],
                    CURSOR,
                    CONFIG.char_h - 1,
                    0xFFFFFFFF,
                ));

                CURSOR += 1;

                if CURSOR == CONFIG.char_w {
                    shift_up();
                    CURSOR = 0;
                }
            }
        } else if scancode == 0xAA {
            SHIFT = 0;
        }
    }
}

pub fn print_num<T: num_traits::PrimInt + num_traits::AsPrimitive<u8> + From<u8>>(mut num: T) {
    let ten = <T as From<u8>>::from(10);
    let zero = T::zero();

    if num < zero {
        raw_print(b"-");
        num = zero - num;
    }

    let next = num / ten;

    if next > zero {
        print_num(next);
    }

    let rem = (num % ten).as_();
    raw_print(&[rem + b'0']);
}

pub fn print_hex<T: num_traits::PrimInt + num_traits::AsPrimitive<u8>>(mut num: T) {
    let nibbles = size_of::<T>() << 1;

    for _ in 0..nibbles {
        num = num.rotate_left(4);
        let rem = num.as_() & 0xF;
        let char = if rem < 10 {
            rem + b'0'
        } else {
            rem + b'A' - 10
        };

        raw_print(&[char]);
    }
}

const SCANCODE_MAP: [u8; 256] = [
    // 00
    0, 0, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 0, 0,
    // 10
    b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', 0, 0, b'a', b's',
    // 20
    b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, b'#', b'z', b'x', b'c',
    b'v', // 30
    b'b', b'n', b'm', b',', b'.', b'/', 0, 0, 0, b' ', 0, 0, 0, 0, 0, 0, // 40
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 50
    0, 0, 0, 0, 0, 0, b'\\', 0, 0, 0, 0, 0, 0, 0, 0, 0, // 60
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 70
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 80
    0, 0, b'!', b'"', 0, b'$', b'%', b'^', b'&', b'*', b'(', b')', b'_', b'+', 0, 0, // 90
    b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P', b'{', b'}', 0, 0, b'A',
    b'S', // A0
    b'D', b'F', b'G', b'H', b'J', b'K', b'L', b':', b'@', 0, 0, b'~', b'Z', b'X', b'C',
    b'V', // B0
    b'B', b'N', b'M', b'<', b'>', b'?', 0, 0, 0, b' ', 0, 0, 0, 0, 0, 0, // C0
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // D0
    0, 0, 0, 0, 0, 0, b'|', 0, 0, 0, 0, 0, 0, 0, 0, 0, // E0
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F0
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

const FONT_DATA: &'static [u8] = include_bytes!("../assets/font");
