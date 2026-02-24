use crate::common::{PagingLevel, PagingMode, PhysAddr, PhysWidth, VirtAddr};
use crate::mmu::paging::{CommonMappedEntry, PageTable, PageTableEntry};
use crate::mmu::view::{DirectoryFlags, EntryView, LeafFlags, PagingAttributesViewer};
use crate::pmm::FrameAllocator;

use super::builder::{EntryBuilder, ViewOrBuilder, view_or_builder};

use core::{ptr, result};

// TODO: Move the Mapper into a constructor.
// The real mapper should utilize HHDM

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    L4Allocation,
    TableAllocation,
    MisalignedAddress,
    InvalidLevel,
    EntryInUse,
}

pub type Result<T> = result::Result<T, Error>;

// Mapper for bootstrap (building initial version)
// A mapper for reading could be cool
// AT const?
pub struct Mapper<'a, FA: FrameAllocator> {
    pml4: PhysAddr<PageTable>,
    fa: &'a mut FA,
    width: PhysWidth,
}

// TODO:? ?
pub struct HelperStats {
    pub three: usize,
    pub two: usize,
    pub one: usize
}

// Map (different sizes, contigious, etc)
// Unmap
//
// Map range, unmap range
// update flags,
// TLB flushes (irrelevant for mapper constructor)

// Safety: Assumes identity mapped address space.
unsafe fn alloc_zerod_table_im<'a, FA: FrameAllocator>(
    fa: &'a mut FA,
    width: PhysWidth,
) -> Option<PhysAddr<PageTable>> {
    // Frame allocator just should know the width..
    let pt = fa.allocate_frame::<PageTable>(width)?;
    // TODO: should static assert page table size

    unsafe {
        let mut_ptr = pt
            .clone()
            .as_im_virt::<{ PagingMode::FourLevel }>()
            .as_mut_ptr();

        // Size = count * size_of::<T>()
        let count = 1;
        let data = 0;
        ptr::write_bytes(mut_ptr, data, count);
    }

    Some(pt)
}

impl<'a, FA: FrameAllocator> Mapper<'a, FA> {
    pub unsafe fn new(fa: &'a mut FA, width: PhysWidth) -> Result<Self> {
        // Safety: PFN are natrually aligned to 4K
        let pml4 = unsafe { alloc_zerod_table_im(fa, width) }.ok_or(Error::L4Allocation)?;

        Ok(Self { pml4, fa, width })
    }

    pub fn pml4(&'a self) -> &'a PhysAddr<PageTable> {
        &self.pml4
    }

    pub fn allocator(&'a mut self) -> &'a mut FA {
        &mut self.fa
    }

    unsafe fn get_entry<const L: PagingLevel>(
        index_table: PhysAddr<PageTable>,
        va: &VirtAddr<{ PagingMode::FourLevel }, ()>,
    ) -> &mut PageTableEntry {
        unsafe {
            let mut_ptr = index_table
                .as_im_virt::<{ PagingMode::FourLevel }>()
                .as_mut_ptr();
            &mut (*mut_ptr).entries[va.level_index::<L>()]
        }
    }

    unsafe fn get_pointed_table_or_construct_empty(
        &mut self,
        level: PagingLevel,
        pte: &mut PageTableEntry,
    ) -> Result<PhysAddr<PageTable>> {
        let build_leaf = false;
        let view_or_builder_instance =
            view_or_builder::<{ PagingMode::FourLevel }>(pte, level, build_leaf)
                .ok_or(Error::InvalidLevel)?;

        match view_or_builder_instance {
            ViewOrBuilder::View(EntryView::Directory(view)) => {
                Ok(view.get_pointed_physical_address(self.width))
            }
            ViewOrBuilder::Builder(EntryBuilder::Directory(dir)) => {
                let new_pt = unsafe { alloc_zerod_table_im::<FA>(self.fa, self.width) }
                    .ok_or(Error::TableAllocation)?;
                // The flags default to most permissive.
                let _ = dir
                    .finalize(new_pt.clone(), &DirectoryFlags::default())
                    .unwrap();
                Ok(new_pt)
            }
            _ => unreachable!(),
        }
    }

    unsafe fn construct_leaf(
        &mut self,
        addr: PhysAddr<()>,
        flags: &LeafFlags,
        level: PagingLevel,
        pte: &mut PageTableEntry,
    ) -> Result<CommonMappedEntry> {
        let build_leaf = true;
        let view_or_builder_instance =
            view_or_builder::<{ PagingMode::FourLevel }>(pte, level, build_leaf)
                .ok_or(Error::InvalidLevel)?;

        match view_or_builder_instance {
            ViewOrBuilder::View(EntryView::Leaf(_)) => Err(Error::EntryInUse),
            ViewOrBuilder::Builder(EntryBuilder::Leaf(leaf)) => leaf
                .finalize(addr, flags)
                .ok_or(Error::MisalignedAddress)
                .map(|e| CommonMappedEntry(e.get_raw_underlying())),
            _ => unreachable!(),
        }
    }

    unsafe fn next_table<const L: PagingLevel>(
        &mut self,
        index_table: PhysAddr<PageTable>,
        va: &VirtAddr<{ PagingMode::FourLevel }, ()>,
    ) -> Result<PhysAddr<PageTable>> {
        unsafe {
            self.get_pointed_table_or_construct_empty(L, Self::get_entry::<L>(index_table, va))
        }
    }

    pub unsafe fn map_leaf(
        &mut self,
        va: VirtAddr<{ PagingMode::FourLevel }, ()>,
        pa: PhysAddr<()>,
        flags: &LeafFlags,
        level: PagingLevel,
    ) -> Result<CommonMappedEntry> {
        // PagingLevel can be const.. really
        if level == PagingLevel::Five || level == PagingLevel::Four {
            return Err(Error::InvalidLevel);
        }

        // TODO: These next tables can fail, and if they fail you probably can clean up? but its not too
        // much of a problem
        // more important in a real mapper
        // Code can be a bit nicer but whatever
        let pdpt = unsafe { self.next_table::<{ PagingLevel::Four }>(self.pml4.clone(), &va) }?;
        if level == PagingLevel::Three {
            // TODO: alright make this run before the logic but what eve
            let vpn = va.vpn();
            if vpn & 0x000F_FFFF_FFC0_0000u64 != vpn {
                return Err(Error::MisalignedAddress);
            }

            return unsafe {
                self.construct_leaf(
                    pa,
                    flags,
                    level,
                    Self::get_entry::<{ PagingLevel::Three }>(pdpt, &va),
                )
            };
        }

        let pd = unsafe { self.next_table::<{ PagingLevel::Three }>(pdpt, &va) }?;
        if level == PagingLevel::Two {
            let vpn = va.vpn();
            if vpn & 0x000F_FFFF_FFFF_FE00u64 != vpn {
                return Err(Error::MisalignedAddress);
            }

            return unsafe {
                self.construct_leaf(
                    pa,
                    flags,
                    level,
                    Self::get_entry::<{ PagingLevel::Two }>(pd, &va),
                )
            };
        }

        let pt = unsafe { self.next_table::<{ PagingLevel::Two }>(pd, &va) }?;
        unsafe {
            self.construct_leaf(
                pa,
                flags,
                level,
                Self::get_entry::<{ PagingLevel::One }>(pt, &va),
            )
        }
    }

    pub unsafe fn map(
        &mut self,
        mut va: VirtAddr<{ PagingMode::FourLevel }, ()>,
        mut pa: PhysAddr<()>,
        flags: &LeafFlags,
        mut number_of_pages: usize
    ) -> Result<HelperStats> {
        // TODO: the VAS has to be smarter. In case of HHDM, if phys is aligned the pa is aligned.
        const PAGES_IN_1GIB: usize = 512 * 512;
        const MASK_1GIB: u64 = 0x000F_FFFF_FFC0_0000u64;

        const PAGES_IN_2MIB: usize = 512;
        const MASK_2MIB: u64 = 0x000F_FFFF_FFFF_FE00u64;

        let mut stats = HelperStats {
            three: 0,
            two: 0,
            one: 0
        };

        while number_of_pages > 0 {
            if number_of_pages >= PAGES_IN_1GIB {
                let va_aligned = va.vpn() & MASK_1GIB == va.vpn();
                let pa_aligned = pa.pfn() & MASK_1GIB == pa.pfn();
                if va_aligned && pa_aligned {
                    unsafe { self.map_leaf(va.clone(), pa.clone(), flags, PagingLevel::Three) }?;
                    va.set_vpn(va.vpn() + PAGES_IN_1GIB as u64);
                    pa.set_pfn(pa.pfn() + PAGES_IN_1GIB as u64);

                    number_of_pages -= PAGES_IN_1GIB;
                    stats.three += 1;
                    continue;
                }
            }

            if number_of_pages >= PAGES_IN_2MIB {
                let va_aligned = va.vpn() & MASK_2MIB == va.vpn();
                let pa_aligned = pa.pfn() & MASK_2MIB == pa.pfn();
                if va_aligned && pa_aligned {
                    unsafe { self.map_leaf(va.clone(), pa.clone(), flags, PagingLevel::Two) }?;
                    va.set_vpn(va.vpn() + PAGES_IN_2MIB as u64);
                    pa.set_pfn(pa.pfn() + PAGES_IN_2MIB as u64);

                    number_of_pages -= PAGES_IN_2MIB;
                    stats.two += 1;
                    continue;
                }
            }

            unsafe { self.map_leaf(va.clone(), pa.clone(), flags, PagingLevel::One) }?;
            va.set_vpn(va.vpn() + 1);
            pa.set_pfn(pa.pfn() + 1);

            number_of_pages -= 1;
            stats.one += 1;
        }

        Ok(stats)
    }
}
