use crate::types::base::*;

pub const EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: Guid = Guid(
    0x9042a9de,
    0x23dc,
    0x4a38,
    [0x96,0xfb,0x7a,0xde,0xd0,0x80,0x51,0x6a],
);

type ResetFn = efi_fn!(
    this: *mut SimpleTextOutputProtocol,
    extended_verification: bool,
);
type OutputStringFn = efi_fn!(
    this: *mut SimpleTextOutputProtocol,
    string: *const u16,
);

#[repr(C)]
pub struct SimpleTextOutputProtocol {
    pub reset: ResetFn,
    pub output_string: OutputStringFn,
    // More functions...
}
