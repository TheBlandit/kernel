#pragma once

#include <efi.h>

void to_int(CHAR16 buf[16], UINT32 num);

// Input text onto line and return length on enter
uint8_t line_input(EFI_SYSTEM_TABLE *SystemTable, CHAR16 line[255]);

// 0 = match, 1 = do not match
int cmp_str16(CHAR16 null_term[], CHAR16 other[], uintptr_t length);
