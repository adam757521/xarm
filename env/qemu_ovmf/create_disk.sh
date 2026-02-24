#!/bin/bash

if [ -z "$1" ]; then
    echo "Usage: $0 <disk_image>"
    exit 1
fi

DISK="$1"

qemu-img create -f raw "$DISK" 64M
mkfs.vfat -F 32 "$DISK"

echo "Disk created: $DISK"
