#include <efi.h>
#include <efilib.h>
#include <stdint.h>
#include "uefi.h"
#include "../output.h"

static EFI_STATUS init_uefi_screen(EFI_SYSTEM_TABLE *SystemTable);
static EFI_STATUS init_gop(EFI_SYSTEM_TABLE *SystemTable);
static EFI_STATUS exit_boot(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable);
static void loop(EFI_SYSTEM_TABLE *SystemTable);

EFI_STATUS init_uefi(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable) {
    EFI_STATUS Status;
    uefi_call_wrapper(SystemTable->BootServices->SetWatchdogTimer, 4, 0, 0, 0, NULL);
    InitializeLib(ImageHandle, SystemTable);

    Status = init_uefi_screen(SystemTable);
    if (EFI_ERROR(Status))
        return Status;

    loop(SystemTable);

    Status = init_gop(SystemTable);
    if (EFI_ERROR(Status))
        return Status;

    // Errors are printed using output now
    Status = exit_boot(ImageHandle, SystemTable);
    if (EFI_ERROR(Status))
        return Status;

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
    if (Status) return Status;
    return uefi_call_wrapper(SystemTable->ConOut->EnableCursor, 2, SystemTable->ConOut, TRUE);
}

static EFI_STATUS init_gop(EFI_SYSTEM_TABLE *SystemTable) {
    EFI_STATUS Status;
    EFI_HANDLE *HandleBuffer;
    UINTN HandleCount;

    // Locate GOP
    Status = uefi_call_wrapper(SystemTable->BootServices->LocateHandleBuffer, 3, ByProtocol, &gEfiGraphicsOutputProtocolGuid, NULL, &HandleCount, &HandleBuffer);
    if (Status) return Status;

    EFI_GRAPHICS_OUTPUT_PROTOCOL *GOP;
    Status = uefi_call_wrapper(SystemTable->BootServices->HandleProtocol, 3, HandleBuffer[0], &gEfiGraphicsOutputProtocolGuid, (void**)&GOP);
    if (Status) return Status;


    // Try to find a good mode (e.g. 1920x1080 or highest)
    for (UINT32 i = 0; i < GOP->Mode->MaxMode; i++) {
        EFI_GRAPHICS_OUTPUT_MODE_INFORMATION *Info;
        UINTN SizeOfInfo;
        Status = uefi_call_wrapper(GOP->QueryMode, 4, GOP, i, &SizeOfInfo, &Info);
        if (Status) return Status;

        if (Info->HorizontalResolution >= 1024 && Info->VerticalResolution >= 768) {
            Status = uefi_call_wrapper(GOP->SetMode, 2, GOP, i);
            break;
        }
    }

    output_init(
        (uint32_t*)GOP->Mode->FrameBufferBase,
        GOP->Mode->FrameBufferSize,
        GOP->Mode->Info->HorizontalResolution,
        GOP->Mode->Info->VerticalResolution,
        GOP->Mode->Info->PixelsPerScanLine,
        GOP->Mode->Info->PixelFormat
    );

    return Status;
}

static void loop(EFI_SYSTEM_TABLE *SystemTable) {
    Print(L"$ ");

    EFI_INPUT_KEY Key;
    UINTN Index;
    EFI_STATUS Status;

    EFI_BOOT_SERVICES* BootServices = SystemTable->BootServices;
    SIMPLE_INPUT_INTERFACE* ConIn = SystemTable->ConIn;
    // SIMPLE_TEXT_OUTPUT_INTERFACE* ConOut = SystemTable->ConOut;

    uint8_t line_length = 0;
    CHAR16 line[255];

    while (TRUE) {
        uefi_call_wrapper(BootServices->WaitForEvent, 3, 1, &ConIn->WaitForKey, &Index);
        Status = uefi_call_wrapper(ConIn->ReadKeyStroke, 2, ConIn, &Key);

        if (!EFI_ERROR(Status)) {
            switch (Key.UnicodeChar) {
                case L'\0':
                    break;
                case L'\r':
                    if (line_length == 3 && line[0] == L'r' && line[1] == L'u' && line[2] == L'n') {
                        return;
                    }

                    Print(L"\n$ ");
                    line_length = 0;
                    break;
                case L'\b':
                    if (line_length) {
                        line_length--;
                        Print(L"\b");
                    }

                    break;
                default:
                    if (line_length != UINT8_MAX) {
                        line[line_length] = Key.UnicodeChar;
                        line_length++;
                        CHAR16 Input[] = { Key.UnicodeChar, L'\0' };
                        Print(Input);
                    }

            }
        }
    }
}
