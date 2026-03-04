use bitfield::bitfield;

/*Intercept exit codes
0h–89h equal the bit position of the corresponding flag in the
VMCB’s intercept vector*/

#[repr(C)]
pub struct ControlArea {
    pub cr_intercepts: u32,
    pub dr_intercepts: u32,
    pub exception_intercepts: u32,
    // All sort of kinds lol
    pub intercepts: u32,
    pub virtualization_intercepts: u32,
    _reserved: [u8; 44],
    pub iopm_paddr: u64,
    pub msrpm_paddr: u64,
    pub tsc_offset: u64,
    pub guest_asid: u32,
    pub tlb_control: u8,
    _reserved1: u8,
    _reserved2: u16,
    _todo60h: u64,
    pub interrupt_shadow: u64,
    pub exitcode: u64,
    pub exitinfo1: u64,
    pub exitinfo2: u64,
    pub exitintinfo: u64,
    pub nested_paging_enable: u64,
    _reserved3: [u8; 16],
    pub event_injection: u64,
    pub host_cr3: u64,
    pub lbr_virtualization_enable: u64,
    _reserved4: [u8; 832]
}

#[repr(C)]
pub struct SegmentSelector {
    pub selector: u16,
    pub attrib: u16,
    pub limit: u32,
    pub base: u64
}

#[repr(C)]
pub struct StateSaveArea {
    pub es: SegmentSelector,
    pub cs: SegmentSelector,
    pub ss: SegmentSelector,
    pub ds: SegmentSelector,
    pub fs: SegmentSelector,
    pub gs: SegmentSelector,

    pub gdtr: SegmentSelector,
    pub ldtr: SegmentSelector,
    pub idtr: SegmentSelector,
    pub tr: SegmentSelector,

    _reserved: [u8; 43],
    pub cpl: u8,
    _reserved1: u32,
    pub efer: u64,
    _reserved2: [u8; 112],
    pub cr4: u64,
    pub cr3: u64,
    pub cr0: u64,
    pub dr7: u64,
    pub dr6: u64,
    pub rflags: u64,
    pub rip: u64,
    _reserved3: [u8; 0x58],
    pub rsp: u64,
    _reserved4: [u8; 24],
    pub rax: u64,
    pub star: u64,
    pub lstar: u64,
    pub cstar: u64,
    pub sfmask: u64,
    pub kernel_gs_base: u64,
    pub sysenter_cs: u64,
    pub sysenter_esp: u64,
    pub sysenter_eip: u64,
    pub cr2: u64,
    _reserved5: [u8; 32],
    pub guest_pat: u64,
    pub dbgctl: u64,
    pub br_from: u64,
    pub br_to: u64,
    pub last_exception_from: u64,
    pub last_exception_to: u64,
    _reserved6: [u8; 0x968],
}

#[repr(C, align(4096))]
pub struct VMCB {
    pub control_area: ControlArea,
    pub state_save_area: StateSaveArea
}
