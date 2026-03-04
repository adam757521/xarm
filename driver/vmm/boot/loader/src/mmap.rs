use uefi;
use uefi::errors;
use uefi::types::{MemoryType, SystemTable};

use core::{ptr, slice};

// Realloc, Malloc, free
// Can implement Drop aswell but we need st shared state so icba
struct EfiMemoryMapAllocation {
    // TODO: Wtf, this should be an enum..
    // better abstract this shit again could have problem
    pub buffer: *mut uefi::MemoryDescriptor,
    pub page_count: Option<usize>,
}

pub struct EfiMemoryMap {
    // Safety
    pub descriptors: &'static [uefi::MemoryDescriptor],
    pub map_key: usize,
}

fn get_uefi_memory_map(
    st: &mut SystemTable,
    allocation: &mut EfiMemoryMapAllocation,
) -> uefi::Result<EfiMemoryMap> {
    // TODO: remember to attack this function. You have control over the UEFI data.
    let mut map_key: usize = 0;
    let mut memory_map_size: usize = 0;
    let mut desc_size: usize = 0;
    let mut desc_ver: u32 = 0;

    let status = uefi::call_boot!(
        st,
        get_memory_map,
        &mut memory_map_size,
        ptr::null_mut(),
        &mut map_key,
        &mut desc_size,
        &mut desc_ver
    );
    match status {
        Ok(()) => return Err(errors::Error::CompromisedData.into()),
        Err(e) => {
            if e.error != errors::Error::BufferTooSmall {
                return Err(e);
            }
        }
    };

    assert!(desc_size == core::mem::size_of::<uefi::MemoryDescriptor>());
    // Version check for field confusion.

    let mut page_count = match allocation.page_count {
        Some(c) => c,
        None => (memory_map_size + desc_size * 4 + 0xFFF) / 0x1000,
    };

    let mut retries = 3;

    loop {
        // TODO: this logic is somewhat messy, theres likely a cleaner way to do it, but this UEFI
        // memory map setup is inherently messy
        // TODO: properly abstract this cuz this is a vuln waiting to happen
        let mut buf_paddr: u64 = 0;
        if let Some(c) = allocation.page_count {
            let _ = uefi::call_boot!(st, free_pages, allocation.buffer as u64, c);
            allocation.page_count = None;
        }

        // AllocateAnyPages
        uefi::call_boot!(
            st,
            allocate_pages,
            0,
            MemoryType::EfiLoaderData,
            page_count,
            &mut buf_paddr
        )?;

        // Identity-Mapped
        let buf_ptr = buf_paddr as *mut uefi::MemoryDescriptor;
        allocation.buffer = buf_ptr;
        allocation.page_count = Some(page_count);

        let retval = uefi::call_boot!(
            st,
            get_memory_map,
            &mut memory_map_size,
            buf_ptr,
            &mut map_key,
            &mut desc_size,
            &mut desc_ver,
        );

        if retval.is_ok() {
            let elements = memory_map_size / desc_size;
            return Ok(EfiMemoryMap {
                descriptors: unsafe { slice::from_raw_parts(buf_ptr, elements) },
                map_key,
            });
        }

        if retries == 0 {
            let _ = uefi::call_boot!(st, free_pages, allocation.buffer as u64, page_count);
            return Err(retval.err().unwrap());
        }

        retries -= 1;
        page_count += 1;
    }
}

fn calculate_pfn_count(descs: &[uefi::MemoryDescriptor]) -> Option<usize> {
    // By the highest paddress the descriptors go
    descs
        .into_iter()
        .map(|d| ((d.physical_start >> 12) + d.number_of_pages) as usize)
        .max()
}

fn calculate_bitset_pages(descs: &[uefi::MemoryDescriptor], overshoot: usize) -> Option<usize> {
    let bitset_size = calculate_pfn_count(descs).map(|c| (c + 7) / 8)?;
    Some((bitset_size + 4095) / 4096 + overshoot)
}

pub fn get_memory_map(st: &mut SystemTable) -> uefi::Result<(EfiMemoryMap, &'static mut [u64])> {
    let mut retries = 0;

    let mut mmap_alloc = EfiMemoryMapAllocation {
        buffer: 0 as *mut uefi::MemoryDescriptor,
        page_count: None,
    };

    while retries < 5 {
        let first_map = get_uefi_memory_map(st, &mut mmap_alloc)?;

        let first_bitset_pages = calculate_bitset_pages(first_map.descriptors, retries)
            .ok_or(errors::Error::NoMapping.into())?;

        let mut bitset_paddr: u64 = 0;
        uefi::call_boot!(
            st,
            allocate_pages,
            0,
            MemoryType::EfiLoaderData,
            first_bitset_pages,
            &mut bitset_paddr
        )?;

        let final_map = get_uefi_memory_map(st, &mut mmap_alloc)?;
        let final_bitset_pages = calculate_bitset_pages(final_map.descriptors, 0)
            .ok_or(errors::Error::NoMapping.into())?;

        if final_bitset_pages <= first_bitset_pages {
            return Ok((final_map, unsafe {
                slice::from_raw_parts_mut(bitset_paddr as *mut u64, final_bitset_pages * 512)
            }));
        } else {
            uefi::call_boot!(st, free_pages, bitset_paddr, first_bitset_pages)?;
            retries += 1;
        }
    }

    Err(errors::Error::InvalidLanguage.into())
}

