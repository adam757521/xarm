use core::ffi::c_void;

macro_rules! efi_fn {
    ($( $arg_name:ident : $arg_ty:ty ),* $(,)?) => {
        extern "win64" fn( $( $arg_name: $arg_ty ),* ) -> Status
    };
}

pub type VOID = c_void;
pub type Handle = *mut VOID;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guid(pub u32, pub u16, pub u16, pub [u8; 8]);

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Status(pub usize);

#[repr(C)]
pub struct TableHeader {
    // The signature can be used to identify the table type.
    pub signature: [u8; 8],
    // The revision is a version.
    pub revision: u32,
    pub header_size: u32,
    // Helpers here...
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct ConfigurationTable {
    pub vendor_guid: [u8; 16],
    pub vendor_table: *const VOID,
}
