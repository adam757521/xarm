use super::paging::*;
use crate::common::{PagingLevel, PagingMode, PhysAddr, PhysWidth};
use core::mem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryFlags {
    pub present: bool,
    pub rw: bool,
    pub us: bool,
    pub pwt: bool,
    pub pcd: bool,
    pub accessed: bool,
    pub restart: bool,
    pub nx: bool,
}

impl Default for DirectoryFlags {
    // Defaults to most permissive.
    fn default() -> Self {
        Self {
            present: true,
            rw: true,
            us: true,
            pwt: false,
            pcd: false,
            accessed: false,
            restart: false,
            nx: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafFlags {
    pub directory_flags: DirectoryFlags,
    pub dirty: bool,
    pub pat: bool,
    pub global: bool,
    // TODO: better protection key type
    pub protection_key: u8,
}

pub struct DirectoryEntryView<'a>(pub(crate) &'a mut PageTableEntry);

pub struct LeafEntryView<'a> {
    pub(crate) entry: &'a mut CommonMappedEntry,
    // TODO(maintainability): can be better worded, or even abstracted, or even safer
    pub(crate) large_leaf_paddr_start: Option<usize>,
}

pub enum EntryView<'a> {
    Directory(DirectoryEntryView<'a>),
    Leaf(LeafEntryView<'a>),
}

impl PageTableEntry {
    pub(crate) fn get_flags(&self) -> DirectoryFlags {
        DirectoryFlags {
            present: self.present(),
            rw: self.rw(),
            us: self.us(),
            pwt: self.pwt(),
            pcd: self.pcd(),
            accessed: self.accessed(),
            restart: self.restart(),
            nx: self.nx(),
        }
    }

    pub(crate) fn set_flags(&mut self, flags: &DirectoryFlags) {
        // TODO(maintainability): What if I change and miss one?
        self.set_present(flags.present);
        self.set_rw(flags.rw);
        self.set_us(flags.us);
        self.set_pwt(flags.pwt);
        self.set_pcd(flags.pcd);
        self.set_accessed(flags.accessed);
        self.set_restart(flags.restart);
        self.set_nx(flags.nx);
    }
}

impl CommonMappedEntry {
    // FL = First Level
    pub(crate) fn get_flags<const FL: bool>(&self) -> LeafFlags {
        let pat = if FL {
            let entry: &PTe = unsafe { mem::transmute(&*self) };
            entry.pat()
        } else {
            let entry: &CommonLargeLeafMappedEntry = unsafe { mem::transmute(&*self) };
            entry.pat()
        };

        let pte: &PageTableEntry = unsafe { mem::transmute(&*self) };
        LeafFlags {
            directory_flags: pte.get_flags(),
            dirty: self.dirty(),
            pat,
            global: self.global(),
            // TODO: double check the downcast with bitfields.
            protection_key: self.protection_key() as u8,
        }
    }

    pub(crate) fn set_flags<const FL: bool>(&mut self, flags: &LeafFlags) {
        let pte: &mut PageTableEntry = unsafe { mem::transmute(&mut *self) };
        pte.set_flags(&flags.directory_flags);

        self.set_dirty(flags.dirty);
        self.set_global(flags.global);
        self.set_protection_key(flags.protection_key as u64);

        if FL {
            let entry: &mut PTe = unsafe { mem::transmute(&mut *self) };
            entry.set_pat(flags.pat);
        } else {
            let entry: &mut CommonLargeLeafMappedEntry = unsafe { mem::transmute(&mut *self) };
            entry.set_pat(flags.pat);
        };
    }
}

// TODO: Will likely be best of this Viewer will be aware of CPUID paging features and MSRs?
// Safety
// A trait for this is likely overkill. and lowkey discusting
pub trait PagingAttributesViewer {
    // This is a good start. But we need to be able to create new pages, set physical addresses,
    // etc.
    type Target;
    type Flags;

    fn get_raw_underlying(&self) -> u64;
    fn get_pointed_physical_address(&self, width: PhysWidth) -> PhysAddr<Self::Target>;

    // Obviously this is slow, aswell, should build a bitmask but its not important rn
    fn get_flags(&self) -> Self::Flags;
    fn set_flags(&mut self, flags: &Self::Flags);
}

impl PagingAttributesViewer for DirectoryEntryView<'_> {
    type Target = PageTable;
    type Flags = DirectoryFlags;

    fn get_raw_underlying(&self) -> u64 {
        self.0.0
    }

    fn get_pointed_physical_address(&self, width: PhysWidth) -> PhysAddr<PageTable> {
        // The frame is essentially an extended PFN.
        PhysAddr::new(self.0.frame() & width.solely_pfn_mask(), 0, width)
    }

    fn get_flags(&self) -> DirectoryFlags {
        self.0.get_flags()
    }

    fn set_flags(&mut self, flags: &DirectoryFlags) {
        self.0.set_flags(flags)
    }
}

impl PagingAttributesViewer for LeafEntryView<'_> {
    type Target = ();
    type Flags = LeafFlags;

    fn get_raw_underlying(&self) -> u64 {
        self.entry.0
    }

    fn get_pointed_physical_address(&self, width: PhysWidth) -> PhysAddr<()> {
        let pfn_mask = width.solely_pfn_mask();
        let pfn = match self.large_leaf_paddr_start {
            Some(start) => {
                let entry: &CommonLargeLeafMappedEntry = unsafe { mem::transmute(&*self.entry) };
                let resv_trunc_mask = !0u64 << (start - 12);
                (entry.frame() << 1) & pfn_mask & resv_trunc_mask
            }
            None => {
                let entry: &PageTableEntry = unsafe { mem::transmute(&*self.entry) };
                entry.frame() & pfn_mask
            }
        };

        PhysAddr::new(pfn, 0, width)
    }

    fn get_flags(&self) -> LeafFlags {
        match self.large_leaf_paddr_start {
            Some(_) => self.entry.get_flags::<false>(),
            None => self.entry.get_flags::<true>(),
        }
    }

    fn set_flags(&mut self, flags: &LeafFlags) {
        match self.large_leaf_paddr_start {
            Some(_) => self.entry.set_flags::<false>(flags),
            None => self.entry.set_flags::<true>(flags),
        }
    }
}

impl PageTableEntry {
    pub fn view_as_level<'a, const M: PagingMode>(
        &'a mut self,
        level: PagingLevel,
    ) -> Option<EntryView<'a>> {
        // TODO: this could be a generic aswell (level)
        assert!(!match (level, M) {
            (PagingLevel::Five, PagingMode::FiveLevel) => false,
            (PagingLevel::Five, _) => true,
            _ => false,
        });

        if !self.present() {
            return None;
        }

        match level {
            PagingLevel::Five | PagingLevel::Four => {
                Some(EntryView::Directory(DirectoryEntryView(self)))
            }
            PagingLevel::Three | PagingLevel::Two => {
                let cll: &mut CommonLargeLeafMappedEntry = unsafe { mem::transmute(&mut *self) };

                if cll.page_size() {
                    Some(EntryView::Leaf(LeafEntryView {
                        entry: unsafe { mem::transmute(&mut *self) },
                        large_leaf_paddr_start: Some(if level == PagingLevel::Three {
                            30
                        } else {
                            21
                        }),
                    }))
                } else {
                    Some(EntryView::Directory(DirectoryEntryView(&mut *self)))
                }
            }
            PagingLevel::One => Some(EntryView::Leaf(LeafEntryView {
                entry: unsafe { mem::transmute(self) },
                large_leaf_paddr_start: None,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaf_decode() {
        // Should we even check for reserved bits? Its easy using masks.

        enum Flags {
            Directory(DirectoryFlags),
            Leaf(LeafFlags),
        }

        struct Entry {
            pte: PageTableEntry,
            flags: Option<Flags>,
            address: Option<PhysAddr<()>>,
            level: PagingLevel,
        }

        // TODO: reason about reserved guarantees.
        let phys_width = PhysWidth::new(48);
        let table: [Entry; 5] = [
            Entry {
                pte: PageTableEntry(0),
                flags: None,
                address: None,
                level: PagingLevel::One,
            },
            Entry {
                // 4k
                pte: PageTableEntry(0x0000_0000_00AB_C003),
                flags: Some(Flags::Leaf(LeafFlags {
                    directory_flags: DirectoryFlags {
                        present: true,
                        rw: true,
                        us: false,
                        pwt: false,
                        pcd: false,
                        accessed: false,
                        restart: false,
                        nx: false,
                    },
                    dirty: false,
                    pat: false,
                    global: false,
                    protection_key: 0,
                })),
                address: Some(PhysAddr::new(0xABC, 0, phys_width)),
                level: PagingLevel::One,
            },
            Entry {
                // 2m
                pte: PageTableEntry(0x0000_0000_00A0_0083),
                flags: Some(Flags::Leaf(LeafFlags {
                    directory_flags: DirectoryFlags {
                        present: true,
                        rw: true,
                        us: false,
                        pwt: false,
                        pcd: false,
                        accessed: false,
                        restart: false,
                        nx: false,
                    },
                    dirty: false,
                    pat: false,
                    global: false,
                    protection_key: 0,
                })),
                address: Some(PhysAddr::new(0xA00, 0, phys_width)),
                level: PagingLevel::Two,
            },
            Entry {
                // 1g
                pte: PageTableEntry(0x0000_0000_4000_0083),
                flags: Some(Flags::Leaf(LeafFlags {
                    directory_flags: DirectoryFlags {
                        present: true,
                        rw: true,
                        us: false,
                        pwt: false,
                        pcd: false,
                        accessed: false,
                        restart: false,
                        nx: false,
                    },
                    dirty: false,
                    pat: false,
                    global: false,
                    protection_key: 0,
                })),
                address: Some(PhysAddr::new(0x40000, 0, phys_width)),
                level: PagingLevel::Three,
            },
            Entry {
                pte: PageTableEntry(0x0000_0000_00AB_C003),
                flags: Some(Flags::Directory(DirectoryFlags {
                    present: true,
                    rw: true,
                    us: false,
                    pwt: false,
                    pcd: false,
                    accessed: false,
                    restart: false,
                    nx: false,
                })),
                address: Some(PhysAddr::new(0xABC, 0, phys_width)),
                level: PagingLevel::Three,
            },
        ];

        for mut e in table {
            let view_opt = e.pte.view_as_level::<{ PagingMode::FourLevel }>(e.level);
            match view_opt {
                Some(v) => {
                    // No point in the trait if i cant get the... underlying it just makes the code
                    // cleaner

                    match v {
                        EntryView::Directory(dir) => {
                            if let Some(Flags::Directory(f)) = e.flags {
                                assert!(
                                    e.address
                                        == Some(
                                            dir.get_pointed_physical_address(phys_width).cast()
                                        )
                                );
                                assert!(f == dir.get_flags());
                            } else {
                                panic!();
                            }
                        }
                        EntryView::Leaf(leaf) => {
                            if let Some(Flags::Leaf(l)) = e.flags {
                                assert!(
                                    e.address
                                        == Some(leaf.get_pointed_physical_address(phys_width))
                                );
                                assert!(l == leaf.get_flags());
                            } else {
                                panic!();
                            }
                        }
                    }
                }
                None => {
                    assert!(e.flags.is_none())
                }
            }
        }
    }
}
