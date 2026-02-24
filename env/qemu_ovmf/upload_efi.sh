#!/bin/bash

if [ $# -ne 2 ]; then
    echo "Usage: $0 <disk_image> <efi_binary>"
    exit 1
fi

DISK="$1"
EFI_BIN="$2"

if [ ! -f "$DISK" ]; then
    echo "Disk image '$DISK' does not exist."
    exit 1
fi

if [ ! -f "$EFI_BIN" ]; then
    echo "EFI binary '$EFI_BIN' does not exist."
    exit 1
fi

MNT="/tmp/efi_mount"

mkdir -p "$MNT"

sudo mount -o loop "$DISK" "$MNT"
sudo mkdir -p "$MNT/EFI/BOOT"
sudo cp "$EFI_BIN" "$MNT/EFI/BOOT/BOOTX64.EFI"
sudo umount "$MNT"

echo "EFI binary uploaded to $DISK"
