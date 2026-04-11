#pragma once

#include <stdint.h>
#include <efi.h>

struct OutputData {
    uint32_t *buffer;
    uint32_t buffer_size;
    uint32_t width;
    uint32_t height;
    uint32_t pitch;
    EFI_GRAPHICS_PIXEL_FORMAT format;
};

void output_init(struct OutputData data);
void raw_print(char string[]);
void raw_println(char string[]);
int move_cursor();
