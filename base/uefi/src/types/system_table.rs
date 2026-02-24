use super::base::*;
use super::protocols::console::*;
use super::services::{boot::BootServices, runtime::RuntimeServices};

#[repr(C)]
pub struct SystemTable {
    pub header: TableHeader,
    // Wide char.
    pub firmware_vendor: *const u16,
    // Again the version.
    pub firmware_revision: u32,

    pub console_in_handle: Handle,
    pub con_in: *mut VOID,
    pub console_out_handle: Handle,
    pub con_out: *mut SimpleTextOutputProtocol,
    pub standard_error_handle: Handle,
    pub std_err: *mut VOID,

    pub runtime_services: *const RuntimeServices,
    pub boot_services: *const BootServices,
    pub number_of_table_entries: usize,
    pub configuration_table: *const ConfigurationTable,
}
