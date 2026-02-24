use mm::mmu::paging::make_cr3;
use mm::mmu::{DirectoryFlags, LeafFlags};
use mm::{PagingLevel, PagingMode, PhysAddr, PhysWidth, VirtAddr};

use mm::pmm::{BumpBitsetAllocator, FrameAllocator, bitset::BoolBitset};
use mm::vmm::Mapper;
use uefi::{MemoryDescriptor, MemoryType};

pub struct HandoffPayload {
    pub bitset_base: VirtAddr<{ PagingMode::FourLevel}, u64>,
    pub bitset_size_bytes: usize,
    pub image_base: VirtAddr<{ PagingMode::FourLevel}, ()>,
    pub im_image_base: VirtAddr<{ PagingMode::FourLevel}, ()>,
    pub image_size_bytes: usize,
    pub phys_width: PhysWidth
}

pub struct TrampolinePayload {
    pub handoff_payload: HandoffPayload,
    pub cr3: u64,
    pub stack: VirtAddr<{ PagingMode::FourLevel }, ()>,
    pub stack_first_frame: PhysAddr<()>
}

fn punch_out_descs(bitset: &mut BoolBitset, descs: &[MemoryDescriptor], types: &[MemoryType]) {
    // This is an insanely unoptimized way to punch the mmap.
    for desc in descs.iter().filter(|d| types.contains(&d.r#type)) {
        let index = (desc.physical_start >> 12) as usize;
        for i in 0..desc.number_of_pages {
            bitset.clear_at(index + i as usize);
        }
    }
}

pub fn build_ram_bitset<'a>(storage: &'a mut [u64], descs: &[MemoryDescriptor]) -> BoolBitset<'a> {
    storage.fill(!0u64);
    let mut bitset = BoolBitset::new(storage);

    punch_out_descs(&mut bitset, descs, &[
        MemoryType::EfiConventionalMemory,
        MemoryType::EfiBootServicesCode,
        MemoryType::EfiBootServicesData,
        // NOTE: The loader is not direct mapped!
        // MemoryType::EfiLoaderCode,
    ]);

    bitset
}

pub fn punch_bitset_data_and_bios(
    bitset: &mut BoolBitset,
    bitset_pages: usize,
    bitset_pfn_start: usize
) {
    for i in 0..bitset_pages {
        bitset.set_at(bitset_pfn_start + i);
    }

    // First MB
    for i in 0..256 {
        bitset.set_at(i);
    }
}

pub unsafe fn setup(
    bitset: &mut BoolBitset,
    descs: &[MemoryDescriptor],
    phys_width: PhysWidth,
    physical_memory_ranges: &[crate::hhdm::MemoryRange],
) -> uefi::Result<TrampolinePayload> {

    // TODO: Way more reliable to pass this metadata as a parameter. (EFILOADERDATA)
    // Wont be surprised if that causes a crash on real hardware.
    //
    // TODO: im going to leave the whole image as R-X for now.

    let loader_code = descs
        .iter()
        .find(|d| d.r#type == MemoryType::EfiLoaderCode)
        .ok_or(uefi::errors::Error::NotFound.into())?;

    let bitset_data = descs
        .iter()
        // At least 2M of bitset.
        .find(|d| d.r#type == MemoryType::EfiLoaderData && d.number_of_pages > 512)
        .ok_or(uefi::errors::Error::NotFound.into())?;

    let mut allocator = BumpBitsetAllocator::new(bitset);
    let allocator_ptr = &raw mut allocator;

    let mut mapper = unsafe { Mapper::new(&mut allocator, phys_width) }
        .map_err(|_| uefi::errors::Error::OutOfResources.into())?;

    unsafe { 
        mapper.map(
            VirtAddr::<{ PagingMode::FourLevel }, ()>::new(512, 0),
            PhysAddr::<()>::new(bitset_data.physical_start >> 12, 0, phys_width),
            &LeafFlags {
                directory_flags: DirectoryFlags {
                    us: false,
                    nx: true,
                    ..DirectoryFlags::default()
                },
                dirty: false,
                pat: false,
                global: false,
                protection_key: 0,
            },
            bitset_data.number_of_pages as usize
        )
    }.map_err(|_| uefi::errors::Error::NoMapping.into())?;

    let code_vpn = 512 + bitset_data.number_of_pages;
    let code_pfn = loader_code.physical_start >> 12;
    unsafe {
        let pa = PhysAddr::<()>::new(code_pfn, 0, phys_width);
        let flags = LeafFlags {
            directory_flags: DirectoryFlags {
                us: false,
                nx: false,
                ..DirectoryFlags::default()
            },
            dirty: false,
            pat: false,
            global: false,
            protection_key: 0,
        };

        // Relocation
        mapper.map(
            VirtAddr::<{ PagingMode::FourLevel }, ()>::new(code_vpn, 0),
            pa.clone(),
            &flags,
            loader_code.number_of_pages as usize
        ).map_err(|_| uefi::errors::Error::NoMapping.into())?;

        // IM
        mapper.map(
            VirtAddr::<{ PagingMode::FourLevel }, ()>::new(code_pfn, 0),
            pa.clone(),
            &flags,
            loader_code.number_of_pages as usize
        ).map_err(|_| uefi::errors::Error::NoMapping.into())?;
    };

    // Stack Allocation (16K)
    const STACK_PAGE_SIZE: u64 = 4;
    let stack_start_vpn = 0x7FFFFFF;

    let mut stack_start_frame = None;
    for i in 0..STACK_PAGE_SIZE {
        let va = VirtAddr::<{ PagingMode::FourLevel }, ()>::new(stack_start_vpn - i, 0);

        let flags = LeafFlags {
            directory_flags: DirectoryFlags {
                us: false,
                nx: true,
                ..DirectoryFlags::default()
            },
            dirty: false,
            pat: false,
            global: false,
            protection_key: 0,
        };

        // mapper was previously borrowed in previous iteration of loop...
        // Safety: This code is single threaded.
        unsafe {
            let pa = (*allocator_ptr)
                .allocate_frame::<()>(phys_width)
                .ok_or(uefi::errors::Error::OutOfResources.into())?;
            if stack_start_frame.is_none() {
                stack_start_frame = Some(pa.clone());
            }

            mapper
                .map_leaf(va, pa, &flags, PagingLevel::One)
                .map_err(|_| uefi::errors::Error::NoMapping.into())?;
        }
    }

    for range in physical_memory_ranges {
        unsafe {
            mapper.map(
                VirtAddr::<{ PagingMode::FourLevel }, ()>::new(0x8000000 + range.pfn_start, 0),
                PhysAddr::<()>::new(range.pfn_start, 0, phys_width),
                &LeafFlags {
                    directory_flags: DirectoryFlags {
                        us: false,
                        nx: true,
                        ..DirectoryFlags::default()
                    },
                    dirty: false,
                    pat: false,
                    global: false,
                    protection_key: 0,
                },
                range.number_of_pages
            )
        }.map_err(|_| uefi::errors::Error::NoMapping.into())?;
    }

    let cr3 = make_cr3(mapper.pml4());

    Ok(TrampolinePayload {
        handoff_payload: HandoffPayload {
            bitset_base: VirtAddr::<{ PagingMode::FourLevel }, u64>::new(512, 0),
            bitset_size_bytes: (bitset_data.number_of_pages as usize) * 4096,
            image_base: VirtAddr::<{ PagingMode::FourLevel }, ()>::new(code_vpn, 0),
            im_image_base: VirtAddr::<{ PagingMode::FourLevel }, ()>::new(code_pfn, 0),
            image_size_bytes: (loader_code.number_of_pages as usize) * 4096,
            phys_width
        },
        cr3,
        stack_first_frame: stack_start_frame.unwrap(),
        stack: VirtAddr::<{ PagingMode::FourLevel }, ()>::new(stack_start_vpn, 0xFF0),
    })
}

pub unsafe fn handoff(payload: TrampolinePayload, entry: extern "sysv64" fn(&HandoffPayload) -> !) -> ! {
    let rsp = payload.stack.as_canonical_address();
    let handoff_payload_size = size_of::<HandoffPayload>();

    // We are using the identity mapped stack in order to write to the new stack frame.
    // We can only write to the first frame, currently.
    assert!(handoff_payload_size < 0x100);

    // Stack Layout:
    // ------------------
    // |                |
    // | HandoffPayload | (handoff_payload_size, 0x40)
    // |                |
    // ------------------ <- handoff_ptr
    let handoff_ptr = rsp - handoff_payload_size as u64;
    let rsp = handoff_ptr & !0xF;

    let im_rsp_handoff = unsafe { payload.stack_first_frame.as_im_virt::<{ PagingMode::FourLevel }>() }
        .as_canonical_address() + (rsp & 0xFFF);
    
    let entry_address = entry as u64;
    let entry_offset = entry_address - payload.handoff_payload.im_image_base.as_canonical_address();
    let entry_relocated = payload.handoff_payload.image_base.as_canonical_address() + entry_offset;

    unsafe {
        core::arch::asm!(
            "rep movsb",
            "mov rsp, {new_rsp}",
            "mov cr3, {cr3}",
            "mov rdi, rsp",
            "jmp {entry}",
            // Source
            in("rsi") (&payload.handoff_payload) as *const HandoffPayload,
            // Dest
            in("rdi") im_rsp_handoff,
            // Size
            in("rcx") handoff_payload_size,
            new_rsp = in(reg) rsp,
            cr3 = in(reg) payload.cr3,
            entry = in(reg) entry_relocated,
            options(noreturn)
        );
    }
}

