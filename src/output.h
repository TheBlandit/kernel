#pragma once

#include <stdint.h>
#include <efi.h>

void output_init(
    uint32_t *nbuffer,
    uint32_t nbuffer_size,
    uint32_t nwidth,
    uint32_t nheight,
    uint32_t npitch,
    EFI_GRAPHICS_PIXEL_FORMAT nformat
);

void raw_print(char string[]);
void raw_println(char string[]);
int move_cursor();
