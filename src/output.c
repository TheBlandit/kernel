#include "output.h"
#include "font.h"
#include "isr.h"
#include <stdint.h>

static uint32_t *buffer;
static uint32_t buffer_size;
static uint32_t pixel_w;
static uint32_t pixel_h;
static uint32_t char_w;
static uint32_t char_h;
static uint32_t pitch;
static EFI_GRAPHICS_PIXEL_FORMAT format;

static uint32_t cursor_x = 0;

const uint32_t CHAR_H = 16;
const uint32_t CHAR_W = 9;
const uint32_t BACKGROUND = 0x00101010;

void output_init(
    uint32_t *nbuffer,
    uint32_t nbuffer_size,
    uint32_t nwidth,
    uint32_t nheight,
    uint32_t npitch,
    EFI_GRAPHICS_PIXEL_FORMAT nformat
) {
    buffer = nbuffer;
    buffer_size = nbuffer_size;
    pixel_w = nwidth;
    pixel_h = nheight;
    pitch = npitch;
    format = nformat;

    char_w = pixel_w / CHAR_W;
    char_h = pixel_h / CHAR_H;

    for (uint32_t x = 0; x < CHAR_W * char_w; x++) {
        for (uint32_t y = 0; y < CHAR_H * char_h; y++) {
            buffer[x + y * pitch] = BACKGROUND;
        }
    }

    print_num((uint64_t)buffer);
    raw_println("");
}

// Shift all lines up by 1
ISR_SAFE static void shift_up() {
    for (uint32_t y = 0; y < (CHAR_H * (char_h - 1)); y++) {
        uint32_t yy = y * pitch;
        for (uint32_t x = 0; x < (CHAR_W * char_w); x++) {
            uint32_t pos = yy + x;
            buffer[pos] = buffer[pos + pitch * CHAR_H];
        }
    }

    for (uint32_t y = CHAR_H * (char_h - 1); y < CHAR_H * char_h; y++) {
        for (uint32_t x = 0; x < (CHAR_W * char_w); x++) {
            buffer[x + y * pitch] = BACKGROUND;
        }
    }
}

ISR_SAFE void print_char(const uint8_t *glyph, uint32_t x, uint32_t y, uint32_t colour) {
    uint32_t x_base = x * CHAR_W;
    uint32_t y_base = y * CHAR_H;

    for (uint32_t col = 0; col < CHAR_W; col++) {
        uint16_t col_data = glyph[col << 1] + (glyph[(col << 1) + 1] << 8);

        for (uint32_t row = 0; row < CHAR_H; row++) {
            if ((col_data >> row) & 1) {
                buffer[x_base + col + (y_base + row) * pitch] = colour;
            } else {
                buffer[x_base + col + (y_base + row) * pitch] = BACKGROUND;
            }
        }
    }
}

ISR_SAFE void raw_print(char string[]) {
    uint32_t i = 0;
    char current = *string;

    while (current) {
        if (cursor_x >= char_w) {
            cursor_x = 0;
            shift_up();
        }

        print_char(FONT_DATA + (current * 18), cursor_x, char_h - 1, 0xFFFFFFFF);

        current = string[++i];
        cursor_x++;
    }
}

ISR_SAFE void raw_println(char *string) {
    raw_print(string);
    cursor_x = 0;
    shift_up();
}

const char SCANCODE_MAP[256] = {
    // 00
    '\0', '\0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', '\0', '\0',
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[', ']', '\0', '\0', 'a', 's',
    'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '`', '\0', '#', 'z', 'x', 'c', 'v',
    'b', 'n', 'm', ',', '.', '/', '\0', '\0', '\0', ' ', '\0', '\0', '\0', '\0', '\0', '\0',
    // 40
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\\', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    // 80
    '\0', '\0', '!', '"', '\0', '$', '%', '^', '&', '*', '(', ')', '_', '+', '\0', '\0',
    'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', '{', '}', '\0', '\0', 'A', 'S',
    'D', 'F', 'G', 'H', 'J', 'K', 'L', ':', '@', '\0', '\0', '~', 'Z', 'X', 'C', 'V',
    'B', 'N', 'M', '<', '>', '?', '\0', '\0', '\0', ' ', '\0', '\0', '\0', '\0', '\0', '\0',
    // C0
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '|', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
};

ISR_SAFE void input_scancode(uint8_t scancode) {
    static char shift = 0;

    if (scancode < 0x80) {
        scancode |= shift;
        char map = SCANCODE_MAP[scancode];
        if (map == '\0') {
            if (scancode == 0x2A) {
                shift = 0x80;
            } else if (scancode == 0x0E) {
                // Backspace
                if (cursor_x) {
                    cursor_x--;
                    const uint8_t *SPACE = FONT_DATA + ((uintptr_t)' ' * 18);
                    print_char(SPACE, cursor_x, char_h - 1, 0xFFFFFFFF);
                }
            } else if (scancode == 0x1C) {
                shift_up();
                cursor_x = 0;
            } else {
                // Unknown key
            }
        } else {
            print_char(FONT_DATA + map * 18, cursor_x, char_h - 1, 0xFFFFFFFF);

            cursor_x++;
            if (cursor_x == char_w) {
                shift_up();
                cursor_x = 0;
            }
        }
    } else {
        if (scancode == 0xAA) {
            shift = 0;
        }
    }
}

void print_num(uint64_t num) {
    uint64_t next = num / 10;
    if (next)
        print_num(next);

    num = num % 10;
    char string[2] = " ";
    string[0] = num + '0';
    raw_print(string);
}
