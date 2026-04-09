# =============================================
# EFI Application Makefile - Multi-source (src/ → build/)
# =============================================

ARCH      := x86_64
TARGET    := main

# Directories
SRC_DIR   := src
BUILD_DIR := build

# gnu-efi paths (adjust if needed)
EFI_INCLUDE := /usr/include/efi
EFI_LIBS    := /usr/lib

EFI_ARCH    := $(ARCH)
EFI_CRT0    := $(EFI_LIBS)/crt0-efi-$(ARCH).o
EFI_LDS     := $(EFI_LIBS)/elf_$(ARCH)_efi.lds

# Tools
CC      := gcc
LD      := ld
OBJCOPY := objcopy

# Compiler flags
CFLAGS := \
    -c \
    -fno-stack-protector \
    -fpic \
    -fshort-wchar \
    -mno-red-zone \
    -I$(EFI_INCLUDE) \
    -I$(EFI_INCLUDE)/$(EFI_ARCH) \
    -DEFI_FUNCTION_WRAPPER \
    -Wall -Wextra

# Linker flags
LDFLAGS := \
    -nostdlib \
    -znocombreloc \
    -shared \
    -Bsymbolic \
    -L$(EFI_LIBS) \
    -T$(EFI_LDS) \
    $(EFI_CRT0)

# Objcopy flags
OBJCOPY_FLAGS := \
    -j .text -j .sdata -j .data -j .rodata \
    -j .dynamic -j .dynsym -j .rel -j .rela \
    -j .reloc \
    --output-target=efi-app-$(ARCH)

# =============================================
# Automatic source detection
SRCS := $(wildcard $(SRC_DIR)/*.c)
OBJS := $(patsubst $(SRC_DIR)/%.c, $(BUILD_DIR)/%.o, $(SRCS))

# =============================================

all: $(TARGET).efi

# Create build directory
$(BUILD_DIR):
	mkdir -p $@

# Compile each .c file into build/ directory
$(BUILD_DIR)/%.o: $(SRC_DIR)/%.c | $(BUILD_DIR)
	$(CC) $(CFLAGS) $< -o $@

# Link all objects into .so
$(BUILD_DIR)/$(TARGET).so: $(OBJS)
	$(LD) $^ $(LDFLAGS) -lgnuefi -lefi -o $@

# Convert to EFI executable
$(BUILD_DIR)/$(TARGET).efi: $(BUILD_DIR)/$(TARGET).so
	$(OBJCOPY) $(OBJCOPY_FLAGS) $< $@

# ====================== QEMU Run Targets ======================

OVMF_CODE := /usr/share/OVMF/x64/OVMF_CODE.4m.fd
OVMF_VARS := OVMF_VARS.fd
BOOT_DIR := boot

run: $(BUILD_DIR)/$(TARGET).efi $(OVMF_VARS)
	mkdir -p $(BOOT_DIR)/EFI/BOOT
	cp $(BUILD_DIR)/$(TARGET).efi $(BOOT_DIR)/EFI/BOOT/BOOTX64.EFI

	qemu-system-x86_64 \
		-m 512M \
		-cpu qemu64 \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-drive file=fat:rw:$(BOOT_DIR),format=raw,if=virtio \
		-net none \
		-vga std

$(OVMF_VARS):
	cp /usr/share/OVMF/x64/OVMF_VARS.4m.fd $@

clean:
	rm -rf $(BUILD_DIR) *.so *.efi *.fd $(BOOT_DIR) fat.img 

.PHONY: all clean
