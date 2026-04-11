#include <efi.h>
#include <efilib.h>
#include "uefi_helper.h"

static void to_int_rec(CHAR16 **buf, UINT32 num) {
    UINT32 next = num / 10;
    UINT32 rem = num % 10;

    if (next)
        to_int_rec(buf, next);

    **buf = rem + L'0';
    (*buf)++;
}

void to_int(CHAR16 buf[16], UINT32 num) {
    CHAR16 **buffy = &buf;
    to_int_rec(buffy, num);
    **buffy = L'\0';
}

int cmp_str16(CHAR16 null_term[], CHAR16 other[], uintptr_t length) {
    uintptr_t i = 0;
    char current = null_term[0];
    while (current) {
        if (i > length || current != other[i]) {
            return 1;
        }

        current = null_term[++i];
    }

    return (i == length) ? 0 : 1;
}

uint8_t line_input(EFI_SYSTEM_TABLE *SystemTable, CHAR16 line[255]) {
    EFI_INPUT_KEY Key;
    UINTN Index;
    EFI_STATUS Status;

    EFI_BOOT_SERVICES* BootServices = SystemTable->BootServices;
    SIMPLE_INPUT_INTERFACE* ConIn = SystemTable->ConIn;
    // SIMPLE_TEXT_OUTPUT_INTERFACE* ConOut = SystemTable->ConOut;

    uint8_t line_length = 0;

    while (TRUE) {
        uefi_call_wrapper(BootServices->WaitForEvent, 3, 1, &ConIn->WaitForKey, &Index);
        Status = uefi_call_wrapper(ConIn->ReadKeyStroke, 2, ConIn, &Key);

        if (!EFI_ERROR(Status)) {
            switch (Key.UnicodeChar) {
                case L'\0':
                    break;
                case L'\r':
                    Print(L"\n");
                    return line_length;
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
