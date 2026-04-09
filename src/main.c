#include "efibind.h"
#include <efi.h>
#include <efilib.h>

void init_screen(EFI_SYSTEM_TABLE *SystemTable);

EFI_STATUS EFIAPI efi_main(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable) {
    SystemTable->BootServices->SetWatchdogTimer(0, 0, 0, NULL);
    InitializeLib(ImageHandle, SystemTable);
    init_screen(SystemTable);

    EFI_INPUT_KEY Key;
    UINTN Index;
    EFI_STATUS Status;

    EFI_BOOT_SERVICES* BootServices = SystemTable->BootServices;
    SIMPLE_INPUT_INTERFACE* ConIn = SystemTable->ConIn;
    // SIMPLE_TEXT_OUTPUT_INTERFACE* ConOut = SystemTable->ConOut;

    Print(L"$ ");

    while (TRUE) {
        uefi_call_wrapper(BootServices->WaitForEvent, 3, 1, &ConIn->WaitForKey, &Index);
        Status = uefi_call_wrapper(ConIn->ReadKeyStroke, 2, ConIn, &Key);

        if (!EFI_ERROR(Status)) {
            switch (Key.UnicodeChar) {
                case L'\0':
                    break;
                case L'\r':
                    Print(L"\n$ ");
                    break;
                default:
                    {}
                    CHAR16 Input[] = { Key.UnicodeChar, L'\0' };
                    Print(Input);
            }
        }
    }

    return EFI_SUCCESS;
}

void init_screen(EFI_SYSTEM_TABLE *SystemTable) {
    EFI_STATUS Status;

    UINTN MaxSize = 0;
    UINTN MaxI = 0;
    UINTN Columns = 0;
    UINTN Rows = 0;
    INT32 i;

    for (i = 0; i < SystemTable->ConOut->Mode->MaxMode; i++) {
        Status = uefi_call_wrapper(SystemTable->ConOut->QueryMode, 4,
                                   SystemTable->ConOut, i, &Columns, &Rows);
        if (!EFI_ERROR(Status)) {
            UINTN Size = Columns * Rows;
            if (Size > MaxSize) {
                MaxSize = Size;
                MaxI = i;
            }
        }
    }

    uefi_call_wrapper(SystemTable->ConOut->SetMode, 2, SystemTable->ConOut, MaxI);
    uefi_call_wrapper(SystemTable->ConOut->ClearScreen, 1, SystemTable->ConOut);
    uefi_call_wrapper(SystemTable->ConOut->EnableCursor, 2, SystemTable->ConOut, TRUE);
}
