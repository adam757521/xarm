#![no_std]

pub mod _export {
    pub use uefi;
}

pub mod debug;
pub mod handoff;
mod hhdm;
pub mod mmap;
pub use handoff::HandoffPayload;

pub mod core {
    use super::*;

    use uefi;
    use mm::PhysWidth;

    pub unsafe fn init(
        handle: uefi::Handle,
        st: &mut uefi::SystemTable,
    ) -> uefi::Result<handoff::TrampolinePayload> {
        let paging = unsafe { amd64::features::PagingFeatures::detect() };
        let phys_width = PhysWidth::new(paging.physical_address_width() as u8);

        // TODO: i might have to enable NX in EFER
        if !paging.pae() || !paging.nx() || !paging.page1gb() {
            return Err(uefi::errors::Error::Unsupported.into());
        }

        let (uefi_memory_map, bitset_storage) = mmap::get_memory_map(st)?;
        /*
        let code = uefi_memory_map.descriptors
            .iter()
            .find(|d| d.r#type == uefi::MemoryType::EfiLoaderCode)
            .ok_or(uefi::errors::Error::NotFound.into())?;
        debug::print_hex(st, code.physical_start);
        */

        uefi::call_boot!(st, exit_boot_services, handle, uefi_memory_map.map_key)?;
        
        // TODO: can we know for sure that the PFN here is the same as the descriptor?
        let pfn = ((bitset_storage.as_ptr() as u64) >> 12) as usize;
        let pages_for_slice = bitset_storage.len() / 512;

        let mut bitset = handoff::build_ram_bitset(bitset_storage, uefi_memory_map.descriptors);
        let hhdm_map = hhdm::build_hhdm_ram(&bitset).unwrap();
        handoff::punch_bitset_data_and_bios(&mut bitset, pages_for_slice, pfn);

        unsafe {
            handoff::setup(&mut bitset, uefi_memory_map.descriptors, phys_width, hhdm_map)
        }
    }
}
