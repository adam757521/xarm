use bitfield::bitfield;
// TODO: this can have better typing, similar to Simd<const N:usize> where LaneCount>N>:
// SupportedLaneCount

bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct SegmentDescriptor(u128);
    impl Debug;

    pub limit0, set_limit0 : 15, 0;
    pub limit48, set_limit48 : 51, 48;

    pub base16, set_base16 : 39, 16;
    pub base56, set_base56 : 63, 56;
    pub base64, set_base64 : 95, 64;

    pub access_byte, set_access_byte : 47, 40;
    pub raw_flags, set_raw_flags : 55, 52;
}

bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct AccessByte(u8);
    impl Debug;

    pub accessed, set_accessed : 0;
    pub rw, set_rw : 1;
    pub dc, set_dc : 2;
    pub exec, set_exec : 3;
    pub s, set_s : 4;
    pub dpl, set_dpl : 6, 5;
    pub present, set_present : 7;
}

// SystemAccessByte..

bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct DescriptorRawFlags(u8);
    impl Debug;

    // Long-mode code flag
    pub lmcf, set_lmcf : 1;
    pub db, set_db : 2;
    // Page granularity
    pub granularity, set_granularity : 3;
}

#[repr(C)]
pub struct DescriptorFlags {
    pub access_byte: AccessByte,
    pub flags: DescriptorRawFlags
}

impl SegmentDescriptor {
    pub fn get_flags(&self) -> DescriptorFlags {
        DescriptorFlags {
            access_byte: AccessByte(self.access_byte() as u8),
            flags: DescriptorRawFlags(self.raw_flags() as u8)
        }
    }

    pub fn set_flags(&mut self, flags: &DescriptorFlags) { 
        self.set_access_byte(flags.access_byte.0 as u128);
        self.set_raw_flags(flags.flags.0 as u128);
    }

    pub fn get_base(&self) -> u64 {
        todo!()
        /*
        (self.offset16() as u64)
            & ((self.offset32_high() as u64) << 16)
            & ((self.offset64_high() as u64) << 32)*/
    }
    
    pub fn set_base(&mut self, address: u64) {
        self.set_base16((address & 0xFFFFFF) as u128);
        self.set_base56(((address & 0xFF000000) >> 24) as u128);
        self.set_base64(((address & 0xFFFFFFFF00000000) >> 32) as u128);
    }

    pub fn set_limit(&mut self, limit: u64) {
        self.set_limit0((limit & 0xFFFF) as u128);
        self.set_limit48(((limit & 0xF0000) >> 16) as u128);
    }
}

