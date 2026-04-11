#include <efi.h>
#include <efilib.h>
#include "efierr.h"
#include "init/init.h"
#include "output.h"

EFI_STATUS EFIAPI efi_main(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable) {
    if (init(ImageHandle, SystemTable)) {
        while (TRUE) {}

        return EFI_SUCCESS;
    }

    raw_println("Hello World");

    while (TRUE) {}

    return EFI_SUCCESS;
}
