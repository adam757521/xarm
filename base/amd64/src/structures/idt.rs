use bitfield::bitfield;

bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct GateEntry(u128);
    impl Debug;
    pub offset16, set_offset16 : 15, 0;
    pub offset32_high, set_offset32_high : 63, 48;
    pub offset64_high, set_offset64_high : 95, 64;

    pub segment, set_segment : 31, 16;
    pub ist, set_ist : 34, 32;
    pub r#type, set_type : 43, 40;
    pub dpl, set_dpl : 46, 45;
    pub present, set_present : 47;
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GateType {
    Interrupt = 0xE,
    Trap = 0xF
}

impl TryFrom<u8> for GateType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0xE => Ok(GateType::Interrupt),
            0xF => Ok(GateType::Trap),
            _ => Err(()),
        }
    }
}

#[repr(C)]
pub struct GateFlags {
    pub segment: u8,
    pub ist: u8,
    pub r#type: GateType,
    pub dpl: u8,
    pub present: bool
}

impl GateEntry {
    pub fn get_flags(&self) -> Result<GateFlags, ()> {
        Ok(GateFlags {
            segment: self.segment() as u8,
            ist: self.ist() as u8,
            r#type: (self.r#type() as u8).try_into()?,
            dpl: self.dpl() as u8,
            present: self.present()
        })
    }

    pub fn set_flags(&mut self, flags: &GateFlags) { 
        self.set_segment(flags.segment as u128);
        self.set_ist(flags.ist as u128);
        self.set_type((flags.r#type as u8) as u128);
        self.set_dpl(flags.dpl as u128);
        self.set_present(flags.present);
    }

    pub fn get_address(&self) -> u64 {
        (self.offset16() as u64)
            & ((self.offset32_high() as u64) << 16)
            & ((self.offset64_high() as u64) << 32)
    }
    
    pub fn set_address(&mut self, address: u64) {
        self.set_offset16((address & 0xFFFF) as u128);
        self.set_offset32_high(((address & 0xFFFF0000) >> 16) as u128);
        self.set_offset64_high(((address & (0xFFFFFFFF00000000)) >> 32) as u128);
    }
}

#[repr(C, align(4096))]
pub struct InterruptDescriptorTable {
    pub descriptors: [GateEntry; 256]
}
