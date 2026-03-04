#![no_std]

#[derive(Copy, Clone, Debug)]
#[derive(Default)]
pub struct Descriptor(pub u16);

// TODO: Descriptor entries are either an entry or not.
// And naming is bad.
impl Descriptor {
    pub const ENTRY: u16 = 0b1;
    pub const TAG_ENTRY: u16 = Self::ENTRY << 15;

    pub const MASK_DATA: u16 = 0x7FFF;

    pub fn new_entry(offset: u16) -> Self {
        debug_assert!(offset <= Self::MASK_DATA);
        Self(Self::TAG_ENTRY | (offset & Self::MASK_DATA))
    }

    pub fn new_leaf(id: u16) -> Self {
        // ZII - Zero is a leaf.
        debug_assert!(id <= Self::MASK_DATA);
        Self(id & Self::MASK_DATA)
    }

    pub fn new_invalid() -> Self {
        Self(0)
    }
}

#[repr(C, align(64))]
#[derive(Default)]
pub struct Entry {
    pub bitmasks: [u32; 4],
    pub expected: [u32; 4],
    pub entries: [Descriptor; 16]
}
