#pragma once

#include <stdint.h>
#include <efi.h>

struct mem_byte_buffer {
    uintptr_t start;
    uintptr_t size;
};

struct mem_page_buffer {
    uintptr_t start;
    uintptr_t pages;
};

uint32_t mem_init(EFI_MEMORY_DESCRIPTOR *MemoryMap, UINTN MemoryMapSize, UINTN DescriptorSize, struct mem_byte_buffer video_buffer);
