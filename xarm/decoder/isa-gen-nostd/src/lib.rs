#![no_std]

#[derive(Copy, Clone, Debug)]
pub struct DescriptorEntry(pub u16);

// TODO: Descriptor entries are either an entry or not.
// And naming is bad.
impl DescriptorEntry {
    pub const NOT_PRESENT: u16 = 0b00;
    pub const BRANCH: u16 = 0b01;
    pub const LEAF:   u16 = 0b10;
    pub const LOOKUP: u16 = 0b11;

    pub const TAG_NOT_PRESENT: u16 = Self::NOT_PRESENT << 14;
    pub const TAG_BRANCH: u16 = Self::BRANCH << 14;
    pub const TAG_LEAF:   u16 = Self::LEAF << 14;
    pub const TAG_LOOKUP: u16 = Self::LOOKUP << 14;
    pub const MASK_DATA:  u16 = 0x3FFF;

    pub fn new_lookup(offset: u16) -> Self {
        debug_assert!(offset <= Self::MASK_DATA);
        Self(Self::TAG_LOOKUP | (offset & Self::MASK_DATA))
    }

    pub fn new_branch(offset: u16) -> Self {
        debug_assert!(offset <= Self::MASK_DATA);
        Self(Self::TAG_BRANCH | (offset & Self::MASK_DATA))
    }

    pub fn new_leaf(id: u16) -> Self {
        debug_assert!(id <= Self::MASK_DATA);
        Self(Self::TAG_LEAF | (id & Self::MASK_DATA))
    }

    #[inline(always)]
    pub fn unpack(self) -> (u16, u16) {
        let tag = self.0 >> 14;
        let val = self.0 & Self::MASK_DATA;
        (tag, val)
    }
}

#[repr(C, align(64))]
#[derive(Debug, Clone)]
pub enum Descriptor {
    Branch {
        // TODO: Two-ahead, Three-ahead
        bitmask: u32,
        expected: u32,
        then: DescriptorEntry,
        r#else: DescriptorEntry,
    },
    Lookup {
        bitmask: u32,
        hint: u32,
        entries: [DescriptorEntry; 16]
    },
    Empty
}
