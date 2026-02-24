#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Bit {
    One,
    Zero,
    NotOne,
    NotZero
}

pub type BitPattern = [Option<Bit>; 32];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BitRegion {
    pub label: Box<str>,
    pub range: std::ops::Range<usize>
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Instruction {
    pub pattern: BitPattern,
    pub regions: Box<[BitRegion]>,
    pub filters: Box<[std::ops::RangeInclusive<usize>]>,
    pub name: Box<str>
}

impl std::hash::Hash for Instruction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pattern.hash(state)
    }
}
