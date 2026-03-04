use crate::types::base::*;

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
