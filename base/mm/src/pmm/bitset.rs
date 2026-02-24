pub struct BoolBitset<'a> {
    storage: &'a mut [u64],
}

// ok huge pages is going to be a pain.
// either a hierarchy or SIMD or different structure

impl<'a> BoolBitset<'a> {
    pub fn new(storage: &'a mut [u64]) -> Self {
        Self { storage }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    // This can obviously have optimized lookups or free chains but its a good database.
    #[inline(always)]
    pub fn set_at(&mut self, index: usize) {
        let word = index / 64;
        self.storage[word] |= 1 << (index % 64);
    }

    #[inline(always)]
    pub fn clear_at(&mut self, index: usize) {
        let word = index / 64;
        self.storage[word] &= !(1 << (index % 64));
    }

    #[inline]
    pub fn assign_at(&mut self, index: usize, value: bool) {
        // Clear then set based on value
        let word = index / 64;
        let mask = 1 << (index % 64);

        let val_mask = (value as u64).wrapping_neg();
        self.storage[word] = (self.storage[word] & !mask) | (mask & val_mask);
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> bool {
        let word = index / 64;
        let mask = 1 << (index % 64);
        (self.storage[word] & mask) != 0
    }
}
