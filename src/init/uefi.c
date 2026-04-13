#include <efi.h>
#include <efilib.h>
#include <stdint.h>
#include "uefi.h"
#include "../output.h"
#include "uefi_helper.h"
#include "interrupts.h"

static EFI_STATUS init_uefi_screen(EFI_SYSTEM_TABLE *SystemTable);
static EFI_STATUS init_gop(EFI_SYSTEM_TABLE *SystemTable);
static EFI_STATUS exit_boot(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable);

EFI_STATUS init_uefi(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable) {
    EFI_STATUS Status;
    uefi_call_wrapper(SystemTable->BootServices->SetWatchdogTimer, 4, 0, 0, 0, NULL);
    InitializeLib(ImageHandle, SystemTable);

    Status = init_uefi_screen(SystemTable);
    if (EFI_ERROR(Status))
        return Status;

    Status = init_gop(SystemTable);
    if (EFI_ERROR(Status))
        return Status;

    Status = create_idt(SystemTable);
    if (EFI_ERROR(Status))
        return Status;

    // Errors are printed using output now
    Status = exit_boot(ImageHandle, SystemTable);
    if (EFI_ERROR(Status))
        return Status;

    init_interrupts();

    return EFI_SUCCESS;
}

static EFI_STATUS exit_boot(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable) {
    EFI_STATUS Status;

    UINTN Pages = 8;
    EFI_MEMORY_DESCRIPTOR *MemoryMap = NULL;
    UINTN MapKey;
    UINTN DescriptorSize;
    UINT32 DescriptorVersion;

    while (TRUE) {
        Status = uefi_call_wrapper(SystemTable->BootServices->AllocatePages, 4, AllocateAnyPages, EfiBootServicesData, Pages, (EFI_PHYSICAL_ADDRESS*)&MemoryMap);
        UINTN MemoryMapSize = Pages << 12;

        if (EFI_ERROR(Status)) {
            raw_println("UEFI exit allocate pages failure");
            return Status;
        }

        Status = uefi_call_wrapper(SystemTable->BootServices->GetMemoryMap, 5, &MemoryMapSize, MemoryMap, &MapKey, &DescriptorSize, &DescriptorVersion);

        if (EFI_ERROR(Status)) {
            Status = uefi_call_wrapper(SystemTable->BootServices->FreePages, 2, (EFI_PHYSICAL_ADDRESS)MemoryMap, Pages);
            if (EFI_ERROR(Status)) {
                raw_println("UEFI exit free pages failure");
                return Status;
            }
        } else {
            Status = uefi_call_wrapper(SystemTable->BootServices->ExitBootServices, 2, ImageHandle, MapKey);

            if (EFI_ERROR(Status)) {
                Status = uefi_call_wrapper(SystemTable->BootServices->FreePages, 2, (EFI_PHYSICAL_ADDRESS)MemoryMap, Pages);
                if (EFI_ERROR(Status)) {
                    raw_println("UEFI exit free pages failure");
                    return Status;
                }
            } else {
                raw_println("UEFI exit success");
                return EFI_SUCCESS;
            }
        }

        Pages = (MemoryMapSize + 0x1FFF) >> 12; // Round up to nearest page and add 1 more
    }
}

static EFI_STATUS init_uefi_screen(EFI_SYSTEM_TABLE *SystemTable) {
    EFI_STATUS Status;

    Status = uefi_call_wrapper(SystemTable->ConOut->ClearScreen, 1, SystemTable->ConOut);
    if (EFI_ERROR(Status))
        return Status;

    return uefi_call_wrapper(SystemTable->ConOut->EnableCursor, 2, SystemTable->ConOut, TRUE);
}

static EFI_STATUS init_gop(EFI_SYSTEM_TABLE *SystemTable) {
    EFI_STATUS Status;
    EFI_HANDLE *HandleBuffer;
    UINTN HandleCount;

    // Locate GOP
    Status = uefi_call_wrapper(SystemTable->BootServices->LocateHandleBuffer, 3, ByProtocol, &gEfiGraphicsOutputProtocolGuid, NULL, &HandleCount, &HandleBuffer);
    if (EFI_ERROR(Status))
        return Status;

    EFI_GRAPHICS_OUTPUT_PROTOCOL *GOP;
    Status = uefi_call_wrapper(SystemTable->BootServices->HandleProtocol, 3, HandleBuffer[0], &gEfiGraphicsOutputProtocolGuid, (void**)&GOP);
    if (EFI_ERROR(Status))
        return Status;

    Print(L"Output options:\n");

    for (UINT32 i = 0; i < GOP->Mode->MaxMode; i++) {
        EFI_GRAPHICS_OUTPUT_MODE_INFORMATION *Info;
        UINTN SizeOfInfo;
        Status = uefi_call_wrapper(GOP->QueryMode, 4, GOP, i, &SizeOfInfo, &Info);
        if (EFI_ERROR(Status))
            return Status;

        Print(L"%d: %dx%d\n", i, Info->HorizontalResolution, Info->VerticalResolution);
    }

    CHAR16 line[255];
    CHAR16 test[16];
    uint8_t line_length;

    while (TRUE) {
        line_length = line_input(SystemTable, line);
        int success = 0;

        for (UINT32 i = 0; i < GOP->Mode->MaxMode; i++) {
            to_int(test, i);
            if (!cmp_str16(test, line, line_length)) {
                Status = uefi_call_wrapper(GOP->SetMode, 2, GOP, i);
                if (EFI_ERROR(Status))
                    return Status;

                success = 1;
                break;
            }
        }

        if (success)
            break;

        Print(L"Invalid mode\n");
    }

    output_init(
        (uint32_t*)GOP->Mode->FrameBufferBase,
        GOP->Mode->FrameBufferSize,
        GOP->Mode->Info->HorizontalResolution,
        GOP->Mode->Info->VerticalResolution,
        GOP->Mode->Info->PixelsPerScanLine,
        GOP->Mode->Info->PixelFormat
    );

    return EFI_SUCCESS;
}
