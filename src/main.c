#include <efi.h>
#include <efilib.h>
#include "init/uefi.h"
#include "output.h"

EFI_STATUS EFIAPI efi_main(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable) {
    init_uefi(ImageHandle, SystemTable);

    raw_println("Hello World");

    while (TRUE) {

    }

    return EFI_SUCCESS;
}
