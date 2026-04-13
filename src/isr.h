#pragma once

#define ISR_SAFE __attribute__((no_caller_saved_registers)) \
    __attribute((target("general-regs-only")))
