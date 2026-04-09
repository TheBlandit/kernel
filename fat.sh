#!/bin/bash

set -e

DIR="fs"          # ← Change this
IMAGE="kernel.img"             # Output image name
SIZE_MB=64                  # Size in MB (increase as needed)

# Create FAT32 image and copy directory
echo "Creating ${SIZE_MB}MB FAT32 image from ${DIR}..."

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

echo "Done! Image created: $IMAGE"
