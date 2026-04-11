#!/bin/bash

set -e

DIR='boot'
IMAGE='bin/kernel.img'
SIZE_MB=64

# Create blank image
dd if=/dev/zero of="$IMAGE" bs=1M count=$SIZE_MB status=progress

# Format as FAT32
mkfs.fat -F 32 -n "EFI" "$IMAGE"

# Mount and copy files
MNT_DIR='/tmp/kernel_mnt'

rm -r "$MNT_DIR" 2> /dev/null || :
mkdir -p "$MNT_DIR"
sudo mount -o loop "$IMAGE" "$MNT_DIR"
sudo cp -r "$DIR"/* "$MNT_DIR"
sudo umount "$MNT_DIR"
rmdir "$MNT_DIR"
