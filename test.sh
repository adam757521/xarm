#!/bin/bash
set -e

echo "--- Building Hypervisor EFI ---"
cargo build --manifest-path xarm/Cargo.toml --target x86_64-unknown-uefi

echo "--- Uploading to Disk Image ---"
./env/qemu_ovmf/upload_efi.sh \
    env/disk.img \
    target/x86_64-unknown-uefi/debug/xarm.efi

echo "--- Launching QEMU ---"
# DEBUG_GDB=env/debug.gdb
 DEBUG_GDB=env/debug.gdb ./env/qemu_ovmf/run_qemu.sh env/disk.img
