#include "uefi.h"
#include <efi.h>

int init(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable) {
    EFI_STATUS Status = init_uefi(ImageHandle, SystemTable);
    if (EFI_ERROR(Status)) return 1;

    return 0;
}
