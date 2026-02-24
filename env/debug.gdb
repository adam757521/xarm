set disassembly-flavor intel
target remote :1234
# Not a good solution at all. Just a temporary workaround.
# Can either use file, or break at first to check the UEFI base address or something of that sort.
#b *0x7E20A368
#b *0x2201110
b *0x0000000002201610
continue

