#include <efi.h>
#include <efilib.h>
#include "init/init.h"
#include "output.h"

EFI_STATUS EFIAPI efi_main(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable) {
    if (init(ImageHandle, SystemTable)) {
        __asm__ volatile("cli\n\t");

        while (TRUE) {
            __asm__ volatile("hlt\n\t");
        }

        return EFI_SUCCESS;
    }

    raw_println("Hello World");

    while (TRUE) {}

    return EFI_SUCCESS;
}
