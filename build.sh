#!/bin/bash

set -euo pipefail

release=false
run=false

for arg in "$@"; do
    case "$arg" in
        'release')
            release=true
            ;;
        'run')
            run=true
            ;;
        *)
            ;;
    esac
done

mkdir -p fs/EFI/BOOT

if [ "$release" = true ]; then
    cargo build --target x86_64-unknown-uefi --release
    cp target/x86_64-unknown-uefi/release/kernel.efi fs/EFI/BOOT/BOOTX64.EFI
else
    cargo build --target x86_64-unknown-uefi
    cp target/x86_64-unknown-uefi/debug/kernel.efi fs/EFI/BOOT/BOOTX64.EFI
fi

if [ "$run" = true ]; then
    if ! test -e 'OVMF_VARS.fd'; then
        cp /usr/share/OVMF/x64/OVMF_VARS.4m.fd 'OVMF_VARS.fd'
    fi

    qemu-system-x86_64 \
        -m 512M \
        -cpu qemu64 \
        -drive if=pflash,format=raw,readonly=on,file='/usr/share/OVMF/x64/OVMF_CODE.4m.fd' \
        -drive if=pflash,format=raw,file='OVMF_VARS.fd' \
        -drive file=fat:rw:'fs',format=raw,if=virtio \
        -net none \
        -vga std
fi

