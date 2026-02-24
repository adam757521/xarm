//#![cfg_attr(not(test), no_std)]

use core::{arch, hint::black_box};
use isa_gen_nostd::{Descriptor, DescriptorEntry};

pub mod _generated {
    #![allow(non_camel_case_types)]

    core::include!(core::concat!(env!("OUT_DIR"), "/a32.rs"));
}

pub use _generated::InstructionView;
use arch::x86_64::*;

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

#[inline(never)]
pub unsafe fn debug_zmm(val: __m512i, label: &str) {
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
pub unsafe fn debug_ymm(val: __m256i, label: &str) {
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

// We are writing low level wrappers, but we should load all of this from memory aswell.

// TODO: mask for the running instructions, instead of a recursive approach
#[inline(always)]
pub unsafe fn find_leftmost_bit_index(masks: __m256i) -> __m256i {
    // Returns the bit number itself.
    unsafe { 
        _mm256_lzcnt_epi32(masks)
    }
}

#[inline(always)]
pub unsafe fn find_rightmost_bit(masks: __m256i) -> __m256i {
    unsafe {
        let one = _mm256_set1_epi32(1);

        // m2 = (mask ^ (mask & (mask - 1)))
        let m0 = _mm256_sub_epi32(masks, one);
        let m1 = _mm256_and_si256(masks, m0);
        let m2 = _mm256_xor_si256(masks, m1);
        // m2 = rightmost bit

        m2
    }
}

// NOTE: we have a higher probability of the bits being at the top.
// we can penalize in the isa-gen itself to not give us such big differences.

// early exit - as long as you have shit in the thing.
//
// http://www.0x80.pl/notesen/2025-01-05-simd-pdep-pext.html
#[inline(always)]
pub unsafe fn vpext512(words: __m512i, mut bitmasks: __m512i) -> __m512i {
    // words -> i32x16
    // masks -> i32x16
    
    unsafe {
        let one = _mm512_set1_epi32(1);
        let zero = _mm512_setzero_si512();

        let mut out = zero;
        let mut bit = one;

        // first_bit = (mask ^ (mask & (mask - 1)))
        let m0 = _mm512_sub_epi32(bitmasks, one);
        let m1 = _mm512_and_si512(bitmasks, m0);
        let first_bit = _mm512_xor_si512(bitmasks, m1);
        
        out = _mm512_or_si512(
            out, 
            _mm512_min_epu32(
                _mm512_and_si512(
                    words,
                    first_bit
                ),
                bit
            )
        );

        bitmasks = m1;
        bit = _mm512_add_epi32(bit, bit);

        let first_leftmost = _mm512_lzcnt_epi32(first_bit);

        let high_256 = _mm512_extracti32x8_epi32::<1>(first_leftmost);
        let low_256  = _mm512_castsi512_si256(first_leftmost);
        let max_8    = _mm256_max_epi32(low_256, high_256);

        // TODO: verify here.
        let max_4 = _mm256_max_epi32(max_8, _mm256_shuffle_i32x4::<0x4E>(max_8, max_8));
        let max_2 = _mm256_max_epi32(max_4, _mm256_shuffle_epi32::<0x4E>(max_4));
        let max_1 = _mm256_max_epi32(max_2, _mm256_shuffle_epi32::<0xB1>(max_2));

        // TODO: if its 32, it should be a thing, we should add a mask to this function.
        // Is this the best approach? Is this off by one?
        let rem = _mm256_cvtsi256_si32(max_1);
        for _ in 0..rem {
            let m0 = _mm512_sub_epi32(bitmasks, one);
            let m1 = _mm512_and_si512(bitmasks, m0);
            let first_bit = _mm512_xor_si512(bitmasks, m1);
            
            out = _mm512_or_si512(
                out, 
                _mm512_min_epu32(
                    _mm512_and_si512(
                        words,
                        first_bit
                    ),
                    bit
                )
            );

            bitmasks = m1;
            bit = _mm512_add_epi32(bit, bit);
        }

        out
    }
}

#[inline(always)]
pub unsafe fn vpext512_hardcoded(words: __m512i, mut bitmasks: __m512i) -> __m512i {
    // words -> i32x16
    // masks -> i32x16
    
    unsafe {
        let one = _mm512_set1_epi32(1);
        let zero = _mm512_setzero_si512();

        let mut out = zero;
        let mut bit = one;

        for _ in 0..32 {
            let m0 = _mm512_sub_epi32(bitmasks, one);
            let m1 = _mm512_and_si512(bitmasks, m0);
            let first_bit = _mm512_xor_si512(bitmasks, m1);
            
            out = _mm512_or_si512(
                out, 
                _mm512_min_epu32(
                    _mm512_and_si512(
                        words,
                        first_bit
                    ),
                    bit
                )
            );

            bitmasks = m1;
            bit = _mm512_add_epi32(bit, bit);
        }

        out
    }
}

pub struct Branch {
    bitmasks: __m512i,
    expected: __m512i,
    // Low 16 -> then, High 16 -> else
    paths: __m512i
}

#[inline(always)]
pub unsafe fn build_branch(ndxs: __m512i, words: __m512i) -> Branch {
    // ndxs -> u32x16

    // TODO: if a branch is a zero, we can optimize it out with a mask register and stop going
    // TODO: mask pext, load all cache lines into zmm registers

    unsafe {
        let zero = _mm512_setzero_si512();
        //let one = _mm512_set1_epi32(1);

        let decoder_table = &_generated::DECODER_POOL as *const _ as *const i32;
        // Scale by 64.
        let ndxs = _mm512_slli_epi32(ndxs, 6);

        // Structure is Branch.
        // perhaps the things here can be SoA..
        let types = _mm512_i32gather_epi32::<1>(
            ndxs,
            decoder_table
        );
        let bitmasks = _mm512_i32gather_epi32::<1>(
            ndxs,
            decoder_table.add(1)
        );

        // We can make it so the descriptor itself will be friendlier to this and we wouldnt need
        // to do that shit here.
        // we need to use a raw union.
        //let branch_masked = black_box(_mm512_movepi32_mask(_mm512_slli_epi32(types, 31)));
        
        // LOAD INTO 16 ZMMS is the real solution.
        let branch_mask: __mmask16 = _mm512_testn_epi32_mask(types, types);
        //println!("{branch_mask}");

        let branch_bitmasks = _mm512_maskz_mov_epi32(branch_mask, bitmasks);
        let expected = _mm512_mask_i32gather_epi32::<1>(
            zero,
            branch_mask,
            ndxs,
            decoder_table.add(2)
        );
        let mut branches = _mm512_mask_i32gather_epi32::<1>(
            zero,
            branch_mask,
            ndxs,
            decoder_table.add(3)
        );

        let entry_ndxs = vpext512(words, _mm512_maskz_mov_epi32(!branch_mask, bitmasks));

        // We don't care about writing the top 16 bits.
        let entries = _mm512_mask_i32gather_epi32::<1>(
            zero,
            !branch_mask,
            _mm512_add_epi32(entry_ndxs, ndxs),
            decoder_table.add(3)
        );

        branches = _mm512_mask_mov_epi32(branches, !branch_mask, entries);

        Branch {
            bitmasks: branch_bitmasks,
            expected,
            paths: branches
        }
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
