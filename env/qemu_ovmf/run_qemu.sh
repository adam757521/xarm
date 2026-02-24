#!/bin/bash

if [ -z "$1" ]; then
    echo "Usage: $0 <path_to_disk_image>"
    exit 1
fi

DISK_IMAGE="$1"
SCRIPT_DIR=$(dirname "$0")

OVMF_CODE="$SCRIPT_DIR/ovmf/OVMF_CODE.fd"
OVMF_VARS="$SCRIPT_DIR/ovmf/OVMF_VARS.fd"

if [ -n "$DEBUG_GDB" ]; then
    GDB_SCRIPT="$DEBUG_GDB"

    if [ ! -f "$GDB_SCRIPT" ]; then
        echo "Error: GDB script '$GDB_SCRIPT' not found"
        exit 1
    fi

	setsid qemu-system-x86_64 \
		-cpu max \
		-drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
		-drive if=pflash,format=raw,file="$OVMF_VARS" \
		-drive file="$DISK_IMAGE",format=raw \
		-m 2G \
		-s -S &

    QEMU_PID=$!

	gdb -x "$GDB_SCRIPT"
	kill $QEMU_PID
else
	qemu-system-x86_64 \
		-cpu max \
		--enable-kvm \
		-serial stdio \
		-drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
		-drive if=pflash,format=raw,file="$OVMF_VARS" \
		-drive file="$DISK_IMAGE",format=raw \
		-m 2G
fi

