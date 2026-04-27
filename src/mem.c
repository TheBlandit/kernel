#include "mem.h"
#include "output.h"
#include <efi.h>
#include <stdint.h>

static struct mem_page_buffer memory_usage;
static uintptr_t phy_pages;
static uint64_t max_phy_addr;
static uint64_t max_lin_addr;
static uint64_t page_ptr_mask;

enum memory_usage_typ:uint8_t {
    MemUsageUnused = 0,
    MemUsageUEFI,
    MemUsageACPI,
    MemUsageUsage,
    MemUsageKernel,
    MemUsageIDT,
    MemUsageGDT,
    MemUsagePaging,
    MemUsageProcess,
};

static inline uint64_t read_cr3() {
    uint64_t cr3;
    __asm__ volatile("mov %%cr3, %0\n\t" : "=r"(cr3) :: "memory");
    return cr3;
}

static inline void write_cr3(uint64_t cr3) {
    __asm__ volatile("mov %0, %%cr3\n\t" : : "r"(cr3) : "memory");
}

uint32_t mem_init(EFI_MEMORY_DESCRIPTOR *MemoryMap, UINTN MemoryMapSize, UINTN DescriptorSize, struct mem_byte_buffer video_buffer) {
    // Get max physical and linear addresses
    {
        uint32_t eax, ebx, ecx, edx;
        __asm__ volatile("cpuid\n\t" : "=a"(eax), "=b"(ebx), "=c"(ecx), "=d"(edx) : "a"(0x80000008));
        uint8_t phy_addr_bits = eax;
        max_phy_addr = (1 << phy_addr_bits) - 1;
        page_ptr_mask = max_phy_addr & ~0xFFF;
        uint8_t virt_addr_bits = eax >> 8;
        max_lin_addr = (1 << virt_addr_bits) - 1;
    }

    // Get phy_pages
    for (uintptr_t i = 0; i < MemoryMapSize / DescriptorSize; i++) {
        EFI_MEMORY_DESCRIPTOR desc = *(EFI_MEMORY_DESCRIPTOR*)((uintptr_t)MemoryMap + DescriptorSize * i);

        if (desc.VirtualStart) {
            raw_println("Non-identity mapped address");
            return 1;
        }

        if (desc.Type != EfiReservedMemoryType && desc.Type != EfiMemoryMappedIO) {
            uintptr_t pages = (desc.PhysicalStart >> 12) + desc.NumberOfPages;
            if (pages > phy_pages) {
               phy_pages = pages;
            }
        }
    }

    memory_usage.pages = (phy_pages + 0xFFF) >> 12;

    // Get suitable buffer for usage
    {
        char not_found_memory_usage_buffer = 1;

        for (uintptr_t i = 0; i < MemoryMapSize / DescriptorSize; i++) {
            EFI_MEMORY_DESCRIPTOR desc = *(EFI_MEMORY_DESCRIPTOR*)((uintptr_t)MemoryMap + DescriptorSize * i);
            if (desc.Type == EfiConventionalMemory && desc.NumberOfPages >= memory_usage.pages) {
                memory_usage.start = desc.PhysicalStart;
                not_found_memory_usage_buffer = 0;
                break;
            }
        }

        if (not_found_memory_usage_buffer) {
            raw_println("No buffer to place memory usage buffer");
            return 1;
        }
    }

    {
        uintptr_t start_page;
        uint8_t *ptr = (uint8_t*)memory_usage.start;

        // Set entire usage buffer to uefi (so default usage is UEFI which is reserved)
        for (uintptr_t i = 0; i < phy_pages; i++) {
            ptr[i] = MemUsageUEFI;
        }

        // Set buffer usages
        for (uintptr_t i = 0; i < MemoryMapSize / DescriptorSize; i++) {
            EFI_MEMORY_DESCRIPTOR desc = *(EFI_MEMORY_DESCRIPTOR*)((uintptr_t)MemoryMap + DescriptorSize * i);

            uint8_t new_type;

            switch (desc.Type) {
                case EfiConventionalMemory:
                case EfiRuntimeServicesCode:
                case EfiBootServicesCode:
                    new_type = MemUsageUnused;
                    break;
                case EfiLoaderCode:
                case EfiLoaderData:
                    new_type = MemUsageKernel;
                    break;
                default:
                    continue;
            }

            start_page = desc.PhysicalStart >> 12;
            for (uintptr_t page = start_page; page < (start_page + desc.NumberOfPages); page++) {
                if (page < phy_pages) {
                    ptr[page] = new_type;
                } else {
                    break;
                }
            }
        }

        // set usage buffer to have type usage
        start_page = memory_usage.start >> 12;
        for (uintptr_t page = start_page; page < (start_page + memory_usage.pages); page++) {
            ptr[page] = MemUsageUsage;
        }
    }

    raw_print("Memory: ");
    print_num(phy_pages << 2);
    raw_println("KiB");

    return 0;
}

static inline uint8_t add_progress(uint64_t *start, uint64_t new, uint64_t *progress, uint64_t add, uint64_t pages) {
    if (*progress == 0)
        *start = new;

    *progress += add;

    return *progress >= pages;
}

// Find hole (min size of pages) in linear address space (4-level paging)
static int64_t find_linear_hole(uint64_t cr3, uint64_t pages, uint64_t kernel) {
    uint64_t hole_start;
    uint64_t page_progress = 0;

    uint64_t *pml4 = (uint64_t*)cr3;

    for (uint64_t i_pml4e = kernel << 11; i_pml4e < (kernel + 1) << 11; i_pml4e++) {
        uint64_t pml4e = pml4[i_pml4e];
        if (pml4e & 1) {
            uint64_t *pdpt = (uint64_t*)(pml4e & page_ptr_mask);

            for (uint64_t i_pdpte = 0; i_pdpte < 0x1000; i_pdpte++) {
                uint64_t pdpte = pdpt[i_pdpte];
                if (pdpte & 1) {
                    if (pdpte & (1 << 7)) {
                        // Giant page
                        page_progress = 0;
                    } else {
                        uint64_t *pd = (uint64_t*)(pdpte & page_ptr_mask);

                        for (uint64_t i_pde = 0; i_pde < 0x1000; i_pde++) {
                            uint64_t pde = pdpt[i_pde];
                            if (pde & 1) {
                                if (pde & (1 << 7)) {
                                    // Huge page
                                    page_progress = 0;
                                } else {
                                    uint64_t *pt = (uint64_t*)(pde & page_ptr_mask);

                                    for (uint64_t i_pte = 0; i_pte < 0x1000; i_pte++) {
                                        uint64_t pte = pt[i_pte];
                                        if (pte & 1) {
                                            // Page
                                            page_progress = 0;
                                        } else {
                                            if (add_progress(&hole_start, i_pte | (i_pde << 9) | (i_pdpte << 18) | (i_pml4e << 27), &page_progress, 1, pages))
                                                return hole_start;
                                        }
                                    }
                                }
                            } else {
                                if (add_progress(&hole_start, (i_pde << 9) | (i_pdpte << 18) | (i_pml4e << 27), &page_progress, 1 << 9, pages))
                                    return hole_start;
                            }
                        }
                    }
                } else {
                    if (add_progress(&hole_start, (i_pdpte << 18) | (i_pml4e << 27), &page_progress, 1 << 18, pages))
                        return hole_start;
                }
            }
        } else {
            if (add_progress(&hole_start, i_pml4e << 27, &page_progress, 1 << 27, pages))
                return hole_start;
        }
    }

    return -1;
}

intptr_t mem_alloc_user(uintptr_t pages, uint64_t attributes, uintptr_t cr3) {
    // Find hole in virtual mem
    // Then allocate any new page tables
    // Then Allocate the pages

    int64_t hole_start = find_linear_hole(cr3, pages, 0);
    if (hole_start < 0) {
        raw_println("Failed to find hole to allocate memory");
        return -1;
    }



    return -1;
}
