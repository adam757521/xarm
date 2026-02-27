//#![cfg_attr(not(test), no_std)]

use core::{arch, hint::black_box};
use isa_gen_nostd::{Descriptor, Entry};

pub mod _generated {
    #![allow(non_camel_case_types)]

    core::include!(core::concat!(env!("OUT_DIR"), "/a32.rs"));
}

pub use _generated::InstructionView;
use arch::x86_64::*;



#[inline(never)]
unsafe fn debug_zmm(val: __m512i, label: &str) {
    let bytes: [u8; 64] = unsafe { std::mem::transmute(val) };
    println!("--- ZMM DEBUG: {} ---", label);
    
    for row in 0..4 {
        print!("{:02X} | ", row * 16);
        for i in 0..16 {
            let idx = row * 16 + i;
            print!("{:02X} ", bytes[idx]);
            if i == 7 { print!("| "); }
        }
        println!();
    }

    println!("-------------------------------------------------------\n");
}

#[inline(never)]
unsafe fn debug_ymm(val: __m256i, label: &str) {
    let bytes: [u8; 32] = unsafe { std::mem::transmute(val) };
    println!("--- YMM DEBUG: {} ---", label);
    
    for row in 0..2 {
        print!("{:02X} | ", row * 16);
        for i in 0..16 {
            let idx = row * 16 + i;
            print!("{:02X} ", bytes[idx]);
            if i == 7 { print!("| "); }
        }
        println!();
    }

    println!("-------------------------------------------------------\n");
}

#[inline(never)]
unsafe fn debug_xmm(val: __m128i, label: &str) {
    let bytes: [u8; 16] = unsafe { std::mem::transmute(val) };
    println!("--- ZMM DEBUG: {} ---", label);
    
    print!("{:02X} | ", 0);
    for i in 0..16 {
        print!("{:02X} ", bytes[i]);
        if i == 7 { print!("| "); }
    }
    println!();

    println!("-------------------------------------------------------\n");
}

macro_rules! emit_lut_lookup {
    ($reg:expr, $lane:expr, $lane_idx:expr, $ndx:expr) => {
        concat!(
            // Each iteration just needs to know its index and word.
            // A lot of the cycles are spent on getting that information.

            "vpextrd {offset:e}, {indices:x}, ", $lane_idx, "\n",
            "vmovdqa64 {", $reg, "}, [{table}+{offset}]\n",

            // values = VAND(word, bitmasks)
            "vextracti32x4 {zmm_temp:x}, {words:z}, ", $lane, "\n",
            "vpshufd {zmm_temp:x}, {zmm_temp:x}, ", $lane_idx, "\n",
            "vpandd {values:x}, {", $reg, ":x}, {zmm_temp:x}\n",

            //"valignd {zmm_temp:y}, {z", $idx, ":y}, {z", $idx, ":y}, 4\n"
            "vextracti32x4 {zmm_temp:x}, {", $reg, ":y}, ", 1, "\n",
            // vectorizing the little cmps, higher throughput

            // Slow sequence: 16 cycles
            "vpcmpeqd {k_temp}, {values:x}, {zmm_temp:x}\n",
            "kmovd {offset:e}, {k_temp}\n",
            "add {offset:e}, 16\n",
            "vpbroadcastw {zmm_temp}, {offset:e}\n",

            "mov {offset:e}, ", $ndx, "\n", 
            "kmovd {k_temp}, {offset:e}\n",
            // VPERMW is low throughput. Just removing this write makes it 2x faster.
            // Can also use 2 ZMMs with VPERMT2W
            "vpermw {result} {{{k_temp}}}, {zmm_temp}, {", $reg, "}\n",
        )
    }
}

#[inline(always)]
pub unsafe fn semi_vectorized_decode(words: __m512i, indices: __m512i) -> __m256i {
    // words, indices -> u32x16

    // TODO: this solution always uses all 16 entries.
    // NOTE: This solution doesn't fully utilize 512-bit lane power.
    unsafe {
        let result: __m256i;

        // diff reg set moves are slow.
        // vpermw is slow.

        arch::asm!(
            // this should obviously use a cleaner macro
            emit_lut_lookup!("cache_line", 0, 0, 0b1),
            emit_lut_lookup!("cache_line", 0, 1, 0b10),
            emit_lut_lookup!("cache_line", 0, 2, 0b100),
            emit_lut_lookup!("cache_line", 0, 3, 0b1000),

            "valignd {indices}, {indices}, {indices}, 4",
            emit_lut_lookup!("cache_line", 1, 0, 0b10000),
            emit_lut_lookup!("cache_line", 1, 1, 0b100000),
            emit_lut_lookup!("cache_line", 1, 2, 0b1000000),
            emit_lut_lookup!("cache_line", 1, 3, 0b10000000),

            "valignd {indices}, {indices}, {indices}, 4",

            emit_lut_lookup!("cache_line", 2, 0, 0b100000000),
            emit_lut_lookup!("cache_line", 2, 1, 0b1000000000),
            emit_lut_lookup!("cache_line", 2, 2, 0b10000000000),
            emit_lut_lookup!("cache_line", 2, 3, 0b100000000000),

            "valignd {indices}, {indices}, {indices}, 4",

            emit_lut_lookup!("cache_line", 3, 0, 0b1000000000000),
            emit_lut_lookup!("cache_line", 3, 1, 0b10000000000000),
            emit_lut_lookup!("cache_line", 3, 2, 0b100000000000000),
            emit_lut_lookup!("cache_line", 3, 3, 0b1000000000000000),
            
            result = out(zmm_reg) result,

            table = in(reg) &_generated::ENTRIES,
            indices = in(zmm_reg) indices,
            words = in(zmm_reg) words,

            offset = out(reg) _,
            values = out(xmm_reg) _,
            zmm_temp = out(zmm_reg) _,
            k_temp = out(kreg) _,
            
            // There's likely bug in the RA where it only "prefers" to give a zmm_reg
            // It will give you a GPR under pressure.
            cache_line = out(zmm_reg) _,
        );

        result
    }
}

/*
#[inline(always)]
pub unsafe fn hotpath_decode_1(word: u32) -> u16 {
    unsafe {
        let root_idx = arch::x86_64::_pext_u32(word, _generated::ROOT_BITMASK);
        let mut fentry = _generated::ROOT_DESCS.get_unchecked(root_idx as usize);

        loop {
            match fentry {
                Descriptor::Lookup { bitmask, entries, .. } => {
                    let idx = arch::x86_64::_pext_u32(word, *bitmask);
                    let next_entry = *entries.get_unchecked(idx as usize);
                    
                    let (tag, val) = next_entry.unpack();
                    if tag == DescriptorEntry::LEAF {
                        return val;
                    }

                    fentry = _generated::DECODER_POOL.get_unchecked(val as usize);
                },
                Descriptor::Branch { .. } => panic!(),
                _ => core::hint::unreachable_unchecked()
            }
        }
    }
}
*/

#[inline(always)]
pub unsafe fn simd_decode(words: __m512i) -> __m256i {
    unsafe {
        // First iteration can be optimzid.
        let r = semi_vectorized_decode(words, _mm512_set1_epi32(0));
        // TODO: implement the loop 
        //debug_ymm(r, "SIMD DECODE");
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test() {

        unsafe {
        //dbg!(core::mem::transmute::<u16, t::InstructionView>(hotpath_decode(0xE5932008)));
        //dbg!(core::mem::transmute::<u16, t::InstructionView>(hotpath_decode(0b11110001000000010000001000000000)));
        dbg!(core::mem::transmute::<u16, _generated::InstructionView>(hotpath_decode_1(0b11100001010000000000000001110000)));
        }
        panic!()

    }
}
