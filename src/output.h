#pragma once

#include <stdint.h>
#include <efi.h>
#include "isr.h"

void output_init(
    uint32_t *nbuffer,
    uint32_t nbuffer_size,
    uint32_t nwidth,
    uint32_t nheight,
    uint32_t npitch,
    EFI_GRAPHICS_PIXEL_FORMAT nformat
);

ISR_SAFE void raw_print(char string[]);
ISR_SAFE void raw_println(char string[]);
ISR_SAFE void input_scancode(uint8_t scancode);

void print_num(uint64_t num);
