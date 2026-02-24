use bitfield::bitfield;
use crate::cpuid::cpuid;

bitfield! {
    pub struct PagingFeatures(u32);
    impl Debug;

    // Leaf: 01H
    // EDX[13]
    pub pae, _ : 0;
    // EDX[16]
    pub pat, _ : 1;
    // ECX[17]
    pub pcid, _ : 2;

    // Leaf 07H.00H
    // ECX[3]
    pub pku, _ : 3;
    // ECX[7]
    pub cet, _ : 4;
    // ECX[16]
    pub la57, _ : 5;
    // ECX[31]
    pub pks, _ : 6;

    // Leaf 80000001H
    // EDX[20]
    pub nx, _ : 7;
    // EDX[26]
    pub page1gb, _ : 8;

    // Leaf: 80000008H (may be unsupported)
    // EAX[7:0]
    // SEAM can change it 3222 combined manual
    pub physical_address_width, _ : 16, 9;
}

impl PagingFeatures {
    pub unsafe fn detect() -> PagingFeatures {
        // Good luck changing this!
        let mut paging = PagingFeatures(0);

        let result = unsafe { cpuid(0x01, 0x00) };
        paging.0 |= (result.edx >> 13) & 1;
        paging.0 |= (result.edx >> 15) & 2;
        paging.0 |= (result.ecx >> 15) & 4;

        let result = unsafe { cpuid(0x07, 0x00) };
        paging.0 |= result.ecx & 8;
        paging.0 |= (result.ecx >> 3) & 16;
        paging.0 |= (result.ecx >> 11) & 32;
        paging.0 |= (result.ecx >> 25) & 64;

        let result = unsafe { cpuid(0x80000001, 0x00) };
        paging.0 |= (result.edx >> 13) & 128;
        paging.0 |= (result.edx >> 18) & 256;

        let result = unsafe { cpuid(0x80000008, 0x00) };
        paging.0 |= (result.eax << 9) & (0xFF << 9);

        paging
    }
}
