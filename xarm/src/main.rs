#![no_std]
#![no_main]

#![feature(abi_x86_interrupt)]

mod interrupts;

use core::panic::PanicInfo;
use core::slice;

use mm::PagingMode;
use mm::pmm::bitset::BoolBitset;
use mm::pmm::{BumpBitsetAllocator, FrameAllocator};
use amd64::svm::features::{enable_svm, toggle_svme};
use amd64::structures::svm::vmcb::VMCB;
use core::arch::asm;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[boot_macros::entry_point]
extern "sysv64" fn hmain(boot_info: &boot::HandoffPayload) -> ! {
    let bitset_storage: &'static mut [u64] = unsafe {
        slice::from_raw_parts_mut(
            boot_info.bitset_base.clone().as_mut_ptr(),
            boot_info.bitset_size_bytes / size_of::<u64>()
        )
    };

    if unsafe { !enable_svm() } {
        unsafe { asm!("ud2") }
    }
    //unsafe { toggle_svme::<false>() };

    let mut bitset = BoolBitset::new(bitset_storage);
    let mut pfn_allocator = BumpBitsetAllocator::new(&mut bitset);

    let base = pfn_allocator.allocate_frame::<()>(boot_info.phys_width).unwrap().cast::<VMCB>();
    let address = base.as_address();

    let hhdm_virt = unsafe { base.as_hhdm_virt::<{ PagingMode::FourLevel }>() };

    let count = 1;
    let data = 0;
    unsafe {
        core::ptr::write_bytes(hhdm_virt.as_mut_ptr(), data, count);
    }

    unsafe { 
        asm!(
            "vmrun",
            in("rax") address,
        )
    };

    loop {}
}
