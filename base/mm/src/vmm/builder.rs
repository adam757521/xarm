use crate::common::{PagingLevel, PagingMode, PhysAddr};
use crate::mmu::paging::{
    CommonLargeLeafMappedEntry, CommonMappedEntry, PageTable, PageTableEntry,
};
use crate::mmu::{DirectoryEntryView, DirectoryFlags, EntryView, LeafEntryView, LeafFlags};

use core::mem;

pub struct DirectoryBuilder<'a> {
    result: &'a mut PageTableEntry,
}

pub struct LeafBuilder<'a> {
    result: &'a mut CommonMappedEntry,
    level: PagingLevel,
}

pub enum EntryBuilder<'a> {
    Directory(DirectoryBuilder<'a>),
    Leaf(LeafBuilder<'a>),
}

pub enum ViewOrBuilder<'a> {
    View(EntryView<'a>),
    Builder(EntryBuilder<'a>),
}

impl<'a> DirectoryBuilder<'a> {
    pub fn finalize(
        self,
        pt: PhysAddr<PageTable>,
        flags: &DirectoryFlags,
    ) -> Option<DirectoryEntryView<'a>> {
        if pt.offset() != 0 {
            return None;
        }

        self.result.0 = 0;
        self.result.set_flags(flags);
        self.result.set_frame(pt.pfn());
        Some(DirectoryEntryView(self.result))
    }
}

impl<'a> LeafBuilder<'a> {
    pub fn finalize(self, addr: PhysAddr<()>, flags: &LeafFlags) -> Option<LeafEntryView<'a>> {
        if addr.offset() != 0 {
            return None;
        }

        // MAXPHYSADDR: 52, 9 bits each level
        let (pfn_mask, paddr_start) = match self.level {
            PagingLevel::Three => (0x000F_FFFF_FFC0_0000u64, Some(30)),
            PagingLevel::Two => (0x000F_FFFF_FFFF_FE00u64, Some(21)),
            PagingLevel::One => (0x000F_FFFF_FFFF_FFFFu64, None),
            _ => unreachable!(),
        };

        let addr_pfn = addr.pfn();
        if addr_pfn & pfn_mask != addr_pfn {
            return None;
        }

        self.result.0 = 0;

        if self.level == PagingLevel::One {
            self.result.set_flags::<true>(flags);

            let entry: &mut PageTableEntry = unsafe { mem::transmute(&mut *self.result) };
            entry.set_frame(addr_pfn);
        } else {
            self.result.set_flags::<false>(flags);

            let entry: &mut CommonLargeLeafMappedEntry =
                unsafe { mem::transmute(&mut *self.result) };
            entry.set_page_size(true);
            entry.set_frame(addr_pfn >> 1);
        }

        Some(LeafEntryView {
            entry: self.result,
            large_leaf_paddr_start: paddr_start,
        })
    }
}

pub fn view_or_builder<'a, const M: PagingMode>(
    pte: &'a mut PageTableEntry,
    level: PagingLevel,
    build_leaf: bool,
) -> Option<ViewOrBuilder<'a>> {
    if level == PagingLevel::One && !build_leaf {
        return None;
    }

    if build_leaf && (level == PagingLevel::Five || level == PagingLevel::Four) {
        return None;
    }

    // Thank you borrow checker. Very cool.
    Some(if pte.present() {
        pte.view_as_level::<M>(level)
            .map(ViewOrBuilder::View)
            .unwrap()
    } else {
        if build_leaf {
            ViewOrBuilder::Builder(EntryBuilder::Leaf(LeafBuilder {
                result: unsafe { mem::transmute(&mut *pte) },
                level,
            }))
        } else {
            ViewOrBuilder::Builder(EntryBuilder::Directory(DirectoryBuilder { result: pte }))
        }
    })

    /*
    Some(if let Some(view) = pte.view_as_level::<M>(level) {
        ViewOrBuilder::View(view)
    } else {
        if build_leaf {
            ViewOrBuilder::Builder(EntryBuilder::Leaf(LeafBuilder {
                result: unsafe { mem::transmute(&mut *pte) },
                level
            }))
        } else {
            ViewOrBuilder::Builder(EntryBuilder::Directory(DirectoryBuilder {
                result: pte,
            }))
        }
    })
    */
}
