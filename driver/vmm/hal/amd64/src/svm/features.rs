use core::arch::asm;
use crate::cpuid::cpuid;

pub unsafe fn toggle_svme<const E: bool>() {
    if E {
        unsafe {
            // EFER
            asm!(
                "mov ecx, 0xC0000080",
                "rdmsr",
                "bts eax, 12",
                "wrmsr",
                out("ecx") _,
                out("eax") _,
                out("edx") _,
            )
        }
    } else {
        unsafe {
            asm!(
                "mov ecx, 0xC0000080",
                "rdmsr",
                "btr eax, 12",
                "wrmsr",
                out("ecx") _,
                out("eax") _,
                out("edx") _,
            )
        }
    };
}

pub unsafe fn is_svm_available() -> bool {
    let result = unsafe { cpuid(0x80000001, 0x00) };
    if result.ecx & 0b100 == 0 {
        return false;
    }

    let mut msr_low: u32;

    unsafe {
        asm!(
            "mov ecx, 0xC0010114",
            "rdmsr",
            out("ecx") _,
            out("eax") msr_low,
            out("edx") _,
        )
    }

    // 4th
    if msr_low & 0x1000 == 0x1000 {
        return false;
    }

    // can also check SVML in 8000000A EDX 

    return true;
}

pub unsafe fn enable_svm() -> bool {
    unsafe {
        if !is_svm_available() { return false };
        toggle_svme::<true>();
    };

    true
}
