#include "output.h"
#include "efiprot.h"
#include "font.h"
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

const uint32_t BACKGROUND = 0x101010;

void output_init(struct OutputData data) {
    buffer = data.buffer;
    buffer_size = data.buffer_size;
    pixel_w = data.width;
    pixel_h = data.height;
    pitch = data.pitch;
    format = data.format;

    char_w = pixel_w / CHAR_W;
    char_h = pixel_h / CHAR_H;

    for (uint32_t x = 0; x < CHAR_W * char_w; x++) {
        for (uint32_t y = 0; y < CHAR_H * char_h; y++) {
            buffer[x + y * pitch] = BACKGROUND;
        }
    }
}

// Shift all lines up by 1
void shift_up() {
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

void raw_print(char string[]) {
    uint32_t i = 0;
    char current = *string;

    while (current) {
        if (cursor_x >= char_w) {
            cursor_x = 0;
            shift_up();
        }

        const uint8_t *char_data = FONT_DATA + (current * 18);

        uint32_t x_base = cursor_x * CHAR_W;
        uint32_t y_base = CHAR_H * (char_h - 1);

        for (uint32_t col = 0; col < CHAR_W; col++) {
            uint16_t col_data = char_data[col << 1] + (char_data[(col << 1) + 1] << 8);

            for (uint32_t row = 0; row < CHAR_H; row++) {
                if ((col_data >> row) & 1) {
                    buffer[x_base + col + (y_base + row) * pitch] = 0xFFFFFFFF;
                }
            }
        }

        current = string[++i];
        cursor_x++;
    }
}

void raw_println(char *string) {
    raw_print(string);
    cursor_x = 0;
    shift_up();
}

