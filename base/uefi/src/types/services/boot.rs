use crate::types::base::*;
use strum::{AsRefStr, EnumCount, FromRepr};

#[repr(u32)]
#[derive(Debug, AsRefStr, FromRepr, EnumCount, Copy, Clone, PartialEq, Eq)]
pub enum MemoryType {
    EfiReservedMemoryType,
    EfiLoaderCode,
    EfiLoaderData,
    EfiBootServicesCode,
    EfiBootServicesData,
    EfiRuntimeServicesCode,
    EfiRuntimeServicesData,
    EfiConventionalMemory,
    EfiUnusableMemory,
    EfiACPIReclaimMemory,
    EfiACPIMemoryNVS,
    EfiMemoryMappedIO,
    EfiMemoryMappedIOPortSpace,
    EfiPalCode,
    EfiPersistentMemory,
    EfiUnacceptedMemoryType,
    EfiMaxMemoryType,
}

#[repr(C)]
pub struct MemoryDescriptor {
    pub r#type: MemoryType,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
    // TODO: this is not always present. But we are targeting x86_64 and its checked at runtime
    // In the memory map
    reserved: u64,
}

type AllocatePagesFn = efi_fn!(
    r#type: u32,
    memory_type: MemoryType,
    pages: usize,
    memory: *mut u64,
);

type FreePagesFn = efi_fn!(
    memory: u64,
    pages: usize,
);

type GetMemoryMapFn = efi_fn!(
    memory_map_size: *mut usize,
    memory_map: *mut MemoryDescriptor,
    map_key: *mut usize,
    descriptor_size: *mut usize,
    descriptor_version: *mut u32,
);
type AllocatePoolFn = efi_fn!(
    pool_type: MemoryType,
    size: usize,
    buffer: *mut *mut VOID,
);
type FreePoolFn = efi_fn!(
    buffer: *mut VOID,
);

type ExitBootServicesFn = efi_fn!(
    image_handle: Handle,
    map_key: usize,
);

type GetNextMonotonicCountFn = efi_fn!(
    count: *mut u64,
);
type StallFn = efi_fn!(
    microseconds: u64,
);
type SetWatchdogTimerFn = efi_fn!(
    timeout: usize,
    watchdog_code: u64,
    data_size: usize,
    watchdog_data: *const u16,
);

type OpenProtocolFn = efi_fn!(
    handle: Handle,
    guid: *const Guid,
    interface: *mut *mut VOID,
    agent_handle: Handle,
    controller_handle: Handle,
    attributes: u32
);

type CalculateCrc32Fn = efi_fn!(
    data: *const VOID,
    data_size: usize,
    crc32: *mut u32,
);

#[repr(C)]
pub struct BootServices {
    pub header: TableHeader,

    // Different functions pointers..
    // Task Priority Services
    _reserved1: [u8; 16],

    // Memory Services
    pub allocate_pages: AllocatePagesFn,
    pub free_pages: FreePagesFn,
    pub get_memory_map: GetMemoryMapFn,
    pub allocate_pool: AllocatePoolFn,
    pub free_pool: FreePoolFn,

    // Event & Timer Services
    _reserved4: [u8; 48],

    // Protocol Handler Services
    _reserved5: [u8; 72],

    // Image Services
    _reserved6: [u8; 32],
    pub exit_boot_services: ExitBootServicesFn,

    // Miscellaneous Services
    pub get_next_monotonic_count: GetNextMonotonicCountFn,
    pub stall: StallFn,
    pub set_watchdog_timer: SetWatchdogTimerFn,

    // Driver Support Services
    _reserved8: [u8; 16],

    // Open and Close Protocol Services
    pub open_protocol: OpenProtocolFn,
    _reserved9: [u8; 16],

    // Library Services
    _reserved10: [u8; 40],

    // 32-bit CRC Services
    pub calculate_crc32: CalculateCrc32Fn,

    // Miscellaneous Services
    _reserved11: [u8; 24],
}
