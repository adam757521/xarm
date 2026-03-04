use crate::common::PhysAddr;
use bitfield::bitfield;

// MMU Paging structures. Intel Vol. 3A 5-31
// Using these structures directly is inherently unsafe because of easy type confusion.

// Flags common to all entries.
// TODO: The frame can be abstracted into different types.
bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct PageTableEntry(u64);
    impl Debug;
    pub present, set_present : 0;
    pub rw, set_rw : 1;
    pub us, set_us : 2;
    pub pwt, set_pwt : 3;
    pub pcd, set_pcd : 4;
    pub accessed, set_accessed: 5;
    pub restart, set_restart : 11;
    pub frame, set_frame : 51, 12;
    pub nx, set_nx : 63;
}

// Flags common to leaf page entries. PDPTE 1GB, PDE 2MB, PTE 4KB
bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct CommonMappedEntry(u64);
    impl Debug;
    pub dirty, set_dirty : 6;
    pub global, set_global : 8;
    pub protection_key, set_protection_key : 62, 59;
}

// Flags common to large leaf page entries. PDPTE 1GB and PDE 2MB
bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct CommonLargeLeafMappedEntry(u64);
    impl Debug;
    pub page_size, set_page_size : 7;
    pub pat, set_pat : 12;
    pub frame, set_frame : 51, 13
}

// First level page table entry. Not to be confused with the generic PageTableEntry
bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct PTe(u64);
    impl Debug;
    pub pat, set_pat : 7;
}

// Incase we would like to allocate a page table on the stack or in BSS.
#[repr(align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

pub fn make_cr3(pml4: &PhysAddr<PageTable>) -> u64 {
    pml4.pfn() << 12
}
