use crate::types::*;

pub const EFI_LOADED_IMAGE_PROTOCOL_GUID: Guid = Guid(
    0x5B1B31A1,
    0x9562,
    0x11d2,
    [0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
);

#[repr(C)]
pub struct LoadedImageProtocol {
    pub revision: u32,
    pub parent_handle: Handle,
    pub system_table: *mut SystemTable,
    pub device_handle: Handle,
    file_path: *mut VOID,
    reserved: *mut VOID,
    pub load_options_size: u32,
    pub load_options: *mut VOID,
    pub image_base: u64,
    pub image_size: u64,
    pub image_code_type: MemoryType,
    pub image_data_type: MemoryType,
    // Unload
}
