#include "interrupts.h"
#include <assert.h>
#include <efi.h>
#include <stdint.h>
#include "../output.h"

const uintptr_t IDT_ENTRIES = 256;
const uint8_t INT_KEYBOARD = 0x21;

struct descriptor {
    uint16_t offset0;
    uint16_t cs;
    uint16_t attributes;
    uint16_t offset16;
    uint32_t offset32;
    uint32_t rsvd;
};

struct int_frame {
    uint64_t rip;
    uint64_t cs;
    uint64_t rflags;
    uint64_t rsp;
    uint64_t ss;
};

static_assert(sizeof(struct descriptor) == 16, "Interrupt descriptor is not 16 bytes");

static struct descriptor *idt;

ISR_SAFE uint8_t inb(uint16_t port) {
    uint8_t value;
    __asm__ volatile (
        "inb %1, %0\n\t"
        : "=a"(value)
        : "Nd"(port)
        : "memory"
    );
    return value;
}

ISR_SAFE void outb(uint16_t port, uint8_t value) {
    __asm__ volatile (
        "outb %0, %1\n\t"
        :
        : "a"(value), "Nd"(port)
        : "memory"
    );
}

__attribute__((interrupt))
static void no_handle(struct int_frame *frame) {
    raw_println("Unhandled interrupt");

    while (TRUE)
        __asm__ volatile (
            "hlt\n\t"
        );
}

__attribute__((interrupt))
static void keyboard_handler(struct int_frame *frame) {
    int8_t scancode = inb(0x60);
    input_scancode(scancode);
    outb(0x20, 0x20);
}

static void set_descriptor(void (*isr)(struct int_frame *frame), uint8_t index) {
    uint16_t cs;
    __asm__ volatile("mov %%cs, %0" : "=r"(cs));

    struct descriptor desc;
    uint64_t ptr = (uint64_t)isr;
    desc.cs = cs;
    desc.rsvd = 0;
    desc.offset0 = ptr;
    desc.offset16 = ptr >> 16;
    desc.offset32 = ptr >> 32;
    desc.attributes = 0b1000111100000000;

    idt[index] = desc;
}

// Called after screen init but before boot leave
EFI_STATUS create_idt(EFI_SYSTEM_TABLE *SystemTable) {
    EFI_STATUS Status;
    EFI_PHYSICAL_ADDRESS buffer;
    const UINTN PAGES = ((IDT_ENTRIES * sizeof(struct descriptor) + 0xFFF) >> 12);
    Status = uefi_call_wrapper(SystemTable->BootServices->AllocatePages, 4, AllocateAnyPages, EfiLoaderData, PAGES, &buffer);
    idt = (struct descriptor*)buffer;

    if (EFI_ERROR(Status)) {
        raw_println("Error allocating memory for IDT");
        return Status;
    }

    for (uintptr_t i = 0; i < IDT_ENTRIES; i++) {
        set_descriptor(no_handle, i);
    }

    set_descriptor(keyboard_handler, INT_KEYBOARD);

    return EFI_SUCCESS;
}

// Called after boot leave
void init_interrupts() {
    const uint16_t IDT_LIMIT = IDT_ENTRIES * sizeof(struct descriptor) - 1;

    struct idtr_ptr {
        uint16_t limit;
        uint64_t ptr;
    } __attribute__((packed));

    struct idtr_ptr ldtr;
    ldtr.limit = IDT_LIMIT;
    ldtr.ptr = (uint64_t)idt;

    __asm__ volatile (
        "lidt %0\n\t"
        "sti\n\t"
        :
        : "m" (ldtr)
        : "memory"
    );

    // Enable PIC
    outb(0x20, 0x11); // Initialize the command port
    outb(0x21, 0x20); // Set vector offset (IRQ0-IRQ7)
    outb(0x21, 0x04); // Set cascading (IRQ2)
    outb(0x21, 0x01); // Set 8086 mode
    outb(0x21, 0xFF); // Mask all interrupts initially

    outb(0x21, inb(0x21) & 0xFD); // Unmask IRQ1 (keyboard)
}
