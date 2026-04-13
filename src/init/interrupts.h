#pragma once

#include <efi.h>

EFI_STATUS create_idt(EFI_SYSTEM_TABLE *SystemTable);
void init_interrupts();
