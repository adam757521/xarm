pub mod bitset;
use bitset::BoolBitset;

use crate::common::{PhysAddr, PhysWidth};

pub trait FrameAllocator {
    fn allocate_frame<T>(&mut self, width: PhysWidth) -> Option<PhysAddr<T>>;
    fn deallocate_frame<T>(&mut self, frame: PhysAddr<T>);
}

pub struct BumpBitsetAllocator<'r, 'a> {
    bitset: &'r mut BoolBitset<'a>,
    cursor: usize,
}

impl<'r, 'a> BumpBitsetAllocator<'r, 'a> {
    pub fn new(bitset: &'r mut BoolBitset<'a>) -> Self {
        Self { bitset, cursor: 0 }
    }
}

impl<'r, 'a> FrameAllocator for BumpBitsetAllocator<'r, 'a> {
    fn allocate_frame<T>(&mut self, width: PhysWidth) -> Option<PhysAddr<T>> {
        // TODO: bitset is assumed to be complacent with width
        let total_bits = self.bitset.len() * 64;
        let start = self.cursor;

        for i in 0..total_bits {
            let index = (start + i) % total_bits;

            if !self.bitset.get(index) {
                self.bitset.set_at(index);

                self.cursor = (index + 1) % total_bits;

                return Some(PhysAddr::new(index as u64, 0, width));
            }
        }

        None
    }

    fn deallocate_frame<T>(&mut self, frame: PhysAddr<T>) {
        // TODO: Bounds checking for nob
        let pfn = frame.pfn() as usize;
        self.bitset.clear_at(pfn);

        if pfn < self.cursor {
            self.cursor = pfn;
        }
    }
}
