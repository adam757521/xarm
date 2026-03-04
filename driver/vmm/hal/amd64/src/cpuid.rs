use core::arch::asm;

pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

pub unsafe fn cpuid(leaf: u32, sub_leaf: u32) -> CpuidResult {
    let mut result = CpuidResult {
        eax: 0,
        ebx: 0,
        ecx: 0,
        edx: 0,
    };

    unsafe {
        // The asm macro makes insanely good sense
        // LLVM wont allow playing with RBX
        // TODO: a better approach here might be saving the rbx on a register, since the RSP can
        // have a weird offset and if we use mem operations it will cause weird behaviour, and for
        // performance reasons but
        // since we take result.ebx as a pointer on a register this is perfectly fine here
        asm!(
            "push rbx",
            "cpuid",
            "mov [{ebx_ptr}], ebx",
            "pop rbx",
            ebx_ptr = in(reg) &mut result.ebx as *mut u32,
            inout("eax") leaf => result.eax,
            inout("ecx") sub_leaf => result.ecx,
            out("edx") result.edx,
        );
    }

    result
}


