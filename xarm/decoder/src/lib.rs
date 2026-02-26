//#![cfg_attr(not(test), no_std)]

use core::{arch, hint::black_box};
use isa_gen_nostd::{Descriptor, Entry};

pub mod _generated {
    #![allow(non_camel_case_types)]

    core::include!(core::concat!(env!("OUT_DIR"), "/a32.rs"));
}

pub use _generated::InstructionView;
use arch::x86_64::*;

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

#[inline(never)]
pub unsafe fn debug_xmm(val: __m128i, label: &str) {
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

#[inline(always)]
pub unsafe fn horizontal_max(values: __m512i) -> i32 {
    // Finding the horizontal max is relatively simple, the trick is folding each time in half.
    // We run a max on the vector, with a shifted version of itself. Think of it as "swapping
    // halves", or ror
    // v = [a, b, c, d]
    // shifted = [c, d, a, b]
    // max = [max(a, c), max(b, d), max(c, a), max(d,b)]

    unsafe { 
        // Align effectively does the swap for us.
        let fold_256 = _mm512_castsi512_si256(_mm512_max_epi32(values, _mm512_alignr_epi32::<8>(values, values)));
        // Swap halves.
        let fold_128 = _mm256_max_epi32(fold_256, _mm256_shuffle_i32x4::<0xB1>(fold_256, fold_256));
        // Unsure if this is the best practice with AVX512 extension.
        let fold_64 = _mm256_max_epi32(fold_128, _mm256_shuffle_epi32::<0x4E>(fold_128));
        let final_fold = _mm256_max_epi32(fold_64, _mm256_shuffle_epi32::<0xB1>(fold_64));

        _mm256_cvtsi256_si32(final_fold)
    }
}

// NOTE: we have a higher probability of the bits being at the top.
// we can penalize in the isa-gen itself to not give us such big differences.

// early exit - as long as you have shit in the thing.
//
// http://www.0x80.pl/notesen/2025-01-05-simd-pdep-pext.html
#[inline(always)]
pub unsafe fn vpext512(words: __m512i, mut bitmasks: __m512i, mask: __mmask16) -> __m512i {
    // words -> i32x16
    // masks -> i32x16
    
    unsafe {
        let one = _mm512_set1_epi32(1);
        let zero = _mm512_setzero_si512();

        let mut out = zero;
        let mut bit = one;

        // first_bit = (mask ^ (mask & (mask - 1)))
        let m0 = _mm512_sub_epi32(bitmasks, one);
        let m1 = _mm512_and_epi32(bitmasks, m0);
        let first_bit = _mm512_mask_xor_epi32(_mm512_set1_epi32(-1), mask, bitmasks, m1);
        
        out = _mm512_or_epi32(
            out, 
            _mm512_min_epu32(
                _mm512_and_epi32(
                    words,
                    first_bit
                ),
                bit
            )
        );

        bitmasks = m1;
        bit = _mm512_add_epi32(bit, bit);

        let leftmost = _mm512_lzcnt_epi32(first_bit);

        // Is this the best approach? Is this off by one?
        let rem = horizontal_max(leftmost);
        for _ in 0..rem {
            let m0 = _mm512_sub_epi32(bitmasks, one);
            let m1 = _mm512_and_epi32(bitmasks, m0);
            let first_bit = _mm512_xor_epi32(bitmasks, m1);
            
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

macro_rules! pzmm {
    ($idx:expr, $mask:expr) => {
        // Vector solution:
        // We have to take all ZMMs, push the XMM to a register, (maybe precomputed masks)
        // We likely can double pump it
        // Might just use align to
        // build here

        //2217 AVX2                :{VEX}  VEXTRACTI128 xmm, ymm, imm8             L:   0.69ns=   2.98c  T:   0.12ns=   0.497c
        // VINSERTI32X4 sounds really good , VPBLENDMD, 
        // CONSTANTS - predefined index map in a register.

        // Incrementing mask - simplest.
        // Blend, as an optimized OR, good to know
        // we only do it one at a time, but we can do better
        // use lower cycle instrucions! like xmms!

        // I need to populate 
        // In theory, building indicies per stage (4 stages) is limited by 2 ZMMs.
        // If we wait each time for these ZMMs we cant parallelize well.
        // But if we "simply" build these ZMMs, we map, from each zmm into:
        //
        // if each time we need two zmms, maybe we can do that shit together
        // They are contigious in memory. we can use that fact.
        //
        // The bitmasks vector theortically is a cast to xmm.
        // VEXTRACTI32x4 for getting high 128 is cheap.
        //
        // The idea before was having 4 bitmasks. is it still easy to do?
        // Yes. this approach allows that to be even easier. 4 grouping

        // Remember that you need to map like this:
        //
        // (0,0)


//4130 AVX512F             :{EVEX} VPSHUFD zmm, zmm, imm8                  L:   0.26ns=   1.13c  T:   0.12ns=   0.499c
//4028 AVX512F             :{EVEX} VPBROADCASTD zmm, xmm                   L:   0.46ns=   1.99c  T:   0.23ns=   0.995c
//3515 AVX512F             :{EVEX} VPORD zmm1, zmm1, zmm2                  L:   0.26ns=   1.10c  T:   0.12ns=   0.498c

//3659 AVX512BW            :{EVEX} VPBLENDMB zmm{k}, zmm, zmm              L:   0.25ns=   1.07c  T:   0.06ns=   0.254c
//3662 AVX512BW            :{EVEX} VPBLENDMW zmm{k}, zmm, zmm              L:   0.25ns=   1.07c  T:   0.06ns=   0.254c
//3665 AVX512F             :{EVEX} VPBLENDMD zmm{k}, zmm, zmm              L:   0.25ns=   1.06c  T:   0.06ns=   0.254c
//3668 AVX512F             :{EVEX} VPBLENDMQ zmm{k}, zmm, zmm              L:   0.25ns=   1.07c  T:   0.06ns=   0.254c

//4132 AVX512F             :{EVEX} VSHUFI32X4 zmm, zmm, zmm, imm8          L:   1.16ns=   4.97c  T:   0.12ns=   0.497c

/*
4159 AVX512VL_VBMI       :{EVEX} VPERMT2B xmm, xmm, xmm                  L:   0.69ns=   2.98c  T:   0.12ns=   0.497c
4160 AVX512VL_VBMI       :{EVEX} VPERMT2B ymm, ymm, ymm                  L:   0.92ns=   3.98c  T:   0.12ns=   0.497c
4161 AVX512_VBMI         :{EVEX} VPERMT2B zmm, zmm, zmm                  L:   1.16ns=   4.98c  T:   0.12ns=   0.498c
4162 AVX512VLBW          :{EVEX} VPERMT2W xmm, xmm, xmm                  L:   0.69ns=   2.98c  T:   0.12ns=   0.497c
4163 AVX512VLBW          :{EVEX} VPERMT2W ymm, ymm, ymm                  L:   0.93ns=   3.98c  T:   0.12ns=   0.498c
4164 AVX512BW            :{EVEX} VPERMT2W zmm, zmm, zmm                  L:   1.16ns=   4.98c  T:   0.12ns=   0.497c
4165 AVX512VL            :{EVEX} VPERMT2D xmm, xmm, xmm                  L:   0.69ns=   2.98c  T:   0.12ns=   0.497c
4166 AVX512VL            :{EVEX} VPERMT2D ymm, ymm, ymm                  L:   0.93ns=   3.98c  T:   0.12ns=   0.498c
4167 AVX512F             :{EVEX} VPERMT2D zmm, zmm, zmm                  L:   1.16ns=   4.99c  T:   0.12ns=   0.497c
4168 AVX512VL            :{EVEX} VPERMT2Q xmm, xmm, xmm                  L:   0.69ns=   2.98c  T:   0.12ns=   0.498c
4169 AVX512VL            :{EVEX} VPERMT2Q ymm, ymm, ymm                  L:   0.92ns=   3.98c  T:   0.12ns=   0.497c
4170 AVX512F             :{EVEX} VPERMT2Q zmm, zmm, zmm                  L:   1.16ns=   4.98c  T:   0.12ns=   0.497c
*/

            // blend dest, src1, src2
            // if the mask is set take from src2, else src1

            // Mask ops are also slow AF.

            //"vpbroadcastd zmm12, xmm", $zmm_src, "\n",
            //"vpmovd2m k3, zmm12\n",
            //"kandw k3, k3, k2\n",
            //"korw k1, k1, k3\n",

            //"mov {offset:e}, ", $idx, "\n",
            //"kmovw k", $mask, ", {offset:e}\n",

            //"vpbroadcastd zmm13 {{k", $mask, "}}, xmm", $zmm_src, "\n",

            // Read from mask, write to another
            // Perm?
            //"vextracti32x4 xmm11, ymm", $zmm_src, ", 0x01"
            //"vpshufd xmm11, xmm", $zmm_src, ", 0x01\n",
            //"vpbroadcastd zmm13 {{k", $mask, "}}, xmm11\n", 
            //"vpord zmm13 {{k", $mask, "}}, zmm13, zmm11\n",

            //"vpshufd xmm11, xmm", $zmm_src, ", 0x02\n",
            //"vpbroadcastd zmm14 {{k", $mask, "}}, xmm11\n", 
            //"vpord zmm14 {{k", $mask, "}}, zmm14, zmm11\n",

            //"vpshufd xmm11, xmm", $zmm_src, ", 0x03\n",
            //"vpbroadcastd zmm15 {{k", $mask, "}}, xmm11\n", 
            //"vpord zmm15 {{k2}}, zmm15, zmm11\n",

        concat!(
            //"vpblendmd zmm15 {{k1}}, zmm15, zmm0\n",

            // obviously we shouldnt get it here.
            // now weve got words
            // weve got indices
            //
            // VAND(word, line:x)
            // VAND, VCMPEQD
            "vpshufd xmm11, {ndx_zmm:x}, 0x01\n",
            "vpandd xmm", $idx, ", xmm", $idx, ", xmm11\n",
            "vextracti32x4 xmm11, ymm", $idx, ", 1\n",
            "vpcmpeqd k1, xmm", $idx, ", xmm11\n",
            // if i get the k1 to a gpr, i got the index
            //"vpextrd {offset:e}, {ndx_zmm:x}, 1\n",
            //"vextracti128 xmm13, ymm0, 1\n",
        )
    }
}

macro_rules! emit_what {
    ($reg:expr, $lane:expr, $lane_idx:expr) => {
        // TODO: handle zeros!!
        concat!(
            "vpextrd {offset:e}, {indices:x}, ", $lane_idx, "\n",
            //"vpandnd {", $reg, "}, {", $reg, "}, {", $reg, "}\n",
            "vmovdqa64 {", $reg, "}, [{table}+{offset}]\n",

            // values = VAND(word, bitmasks)
            "vextracti32x4 {zmm_temp:x}, {words:z}, ", $lane, "\n",
            "vpshufd {zmm_temp:x}, {zmm_temp:x}, ", $lane_idx, "\n",
            "vpandd {values:x}, {", $reg, ":x}, {zmm_temp:x}\n",
            // VCMPEQ(res, expected)
            //"valignd {zmm_temp:y}, {z", $idx, ":y}, {z", $idx, ":y}, 4\n"
            "vextracti32x4 {zmm_temp:x}, {", $reg, ":y}, ", 1, "\n",
            // vectorizing the little cmps, higher throughput
            // TODO: this can use xor/sub.
            "vpcmpeqd {k_temp}, {", $reg, ":x}, {zmm_temp:x}\n",
            // WAW
            "vpmovm2d {zmm_temp}, {k_temp}\n",
            //
            // Permute and index based on it maybe not move
            // result[idx] = line[k+32]
            // get the mask in binary.
        )
    }
}

//macro_rules! emit_thing {
    //($idx: 
//}

#[inline(always)]
pub unsafe fn semi_vectorized_decode(words: __m512i, indices: __m512i) {
    // words, indices -> u32x16

    // NOTE: This solution doesn't fully utilize 512-bit lane power.
    unsafe {
        // ... not sure if we need these
        /*
        let z0: __m512i; let z1: __m512i; let z2: __m512i; let z3: __m512i;
        let z4: __m512i; let z5: __m512i; let z6: __m512i; let z7: __m512i;
        let z8: __m512i; let z9: __m512i; let z10: __m512i; let z11: __m512i;
        let z12: __m512i; let z13: __m512i; let z14: __m512i; let z15: __m512i;*/

        let result: __m512i;
        let t: __m512i;

        // {z0} {z1} {z2} {z3} {z4} {z5} {z6} {z7} {z8} {z9} {z10} {z11} {z12} {z13} {z14} {z15}
        arch::asm!(
            "/* {table} {indices} {words} {offset} {zmm_temp}  {result} {k_temp} {values}*/",
            emit_what!("cache_line", 0, 0),
            emit_what!("cache_line", 0, 1),
            emit_what!("cache_line", 0, 2),
            emit_what!("cache_line", 0, 3),

            "valignd {indices}, {indices}, {indices}, 4",
            emit_what!("cache_line", 1, 0),
            emit_what!("cache_line", 1, 1),
            emit_what!("cache_line", 1, 2),
            emit_what!("cache_line", 1, 3),

            "valignd {indices}, {indices}, {indices}, 4",

            emit_what!("cache_line", 2, 0),
            emit_what!("cache_line", 2, 1),
            emit_what!("cache_line", 2, 2),
            emit_what!("cache_line", 2, 3),

            "valignd {indices}, {indices}, {indices}, 4",

            emit_what!("cache_line", 3, 0),
            emit_what!("cache_line", 3, 1),
            emit_what!("cache_line", 3, 2),
            emit_what!("cache_line", 3, 3),
            
            result = out(zmm_reg) result,

            table = in(reg) &_generated::ENTRIES,
            indices = in(zmm_reg) indices,
            words = in(zmm_reg) words,

            offset = out(reg) _,
            values = out(xmm_reg) _,
            zmm_temp = out(zmm_reg) _,
            k_temp = out(kreg) _,
            
            /*
            z0 = out(zmm_reg) _,
            z1 = out(zmm_reg) _,
            z2 = out(zmm_reg) _,
            z3 = out(zmm_reg) _,
            z4 = out(zmm_reg) _,
            z5 = out(zmm_reg) _,
            z6 = out(zmm_reg) _,
            z7 = out(zmm_reg) _,
            z8 = out(zmm_reg) _,
            z9 = out(zmm_reg) _,
            z10 = out(zmm_reg) _,
            z11 = out(zmm_reg) _,
            z12 = out(zmm_reg) _,
            z13 = out(zmm_reg) _,*/
            // Can't get why the RA gives me GPR if theres pressure

            // This will be renamed, it's just a WAW hazard.
            cache_line = out(zmm_reg) _,
        );

        //debug_zmm(indices, "t");
        //debug_zmm(t, "t");
    }

}

/*
#[inline(always)]
pub unsafe fn playground(ndxs: __m512i, words: __m512i) {
    // ndxs -> u32x16

    unsafe {
        // Building Expected/Branches: Alignment is a big one, can save an instruction maybe by
        // using a bigger size?
        // This is somewhat a problem since we need it as zeros for the lookups.

        let lookup_mask: __mmask16;
        let mut bitmasks: __m512i;
        let mut expected: __m512i;
        let mut branches: __m512i;

        let z0: __m512i; let z1: __m512i; let z2: __m512i; let z3: __m512i;
        let z4: __m512i; let z5: __m512i; let z6: __m512i; let z7: __m512i;
        let z8: __m512i; let z9: __m512i; let z10: __m512i; let z11: __m512i;
        let z12: __m512i; let z13: __m512i; let z14: __m512i; let z15: __m512i;

        // Grouping port usage seemed to work great unintuitively. 4.14 IPC on Zen5
        arch::asm!(
            // Remove this from here
            //"vpslld {ndx_zmm}, {ndx_zmm}, 6",

            // I am almost sure these accesses retire in 7 cycles.
            // Trying to force the CPU to "serialize" the load buffer by basically noping
            // on those zmms gave only a slight cycle increase
            "vmovd {offset:e}, {ndx_zmm:x}",
            "vmovdqa64 zmm16, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 1",
            "vmovdqa64 zmm17, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 2",
            "vmovdqa64 zmm18, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 3",
            "vmovdqa64 zmm19, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 4",
            "vmovdqa64 zmm20, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 5",
            "vmovdqa64 zmm21, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 6",
            "vmovdqa64 zmm22, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 7",
            "vmovdqa64 zmm23, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 8",
            "vmovdqa64 zmm24, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 9",
            "vmovdqa64 zmm25, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 10",
            "vmovdqa64 zmm26, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 11",
            "vmovdqa64 zmm27, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 12",
            "vmovdqa64 zmm28, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 13",
            "vmovdqa64 zmm29, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 14",
            "vmovdqa64 zmm30, [{table}+{offset}]",
            "vpextrd {offset:e}, {ndx_zmm:x}, 15",
            "vmovdqa64 zmm31, [{table}+{offset}]",
            
            /*
            "vpxord zmm15, zmm15, zmm15",
            "vpxord zmm14, zmm14, zmm14",
            "vpxord zmm13, zmm13, zmm13",*/

            /*
            "mov {offset:e}, 1",
            "kmovw k2, {offset:e}",*/

            pzmm!(16, 1),
            pzmm!(17, 2),
            pzmm!(18, 3),
            pzmm!(19, 4),
            pzmm!(20, 5),
            pzmm!(21, 6),
            pzmm!(22, 7),
            pzmm!(23, 1),
            pzmm!(24, 1),
            pzmm!(25, 2),
            pzmm!(26, 3),
            pzmm!(27, 4),
            pzmm!(28, 5),
            pzmm!(29, 6),
            pzmm!(30, 7),
            pzmm!(31, 1),

            //"vpmovd2m k1, zmm10",

            ndx_zmm = in(zmm_reg) ndxs,
            table = in(reg) &_generated::DECODER_POOL,

            // Allocations
            offset = out(reg) _,

            out("k1") lookup_mask,

            // Temporary allocations
            out("k2") _,
            //out("k3") _,
            out("zmm10") _,
            out("zmm11") _,
            out("zmm12") _,

            out("zmm13") bitmasks,
            out("zmm14") expected,
            out("zmm15") branches,

            // Cache line for each instruction.
            out("zmm16") z0,
            out("zmm17") z1,
            out("zmm18") z2,
            out("zmm19") z3,
            out("zmm20") z4,
            out("zmm21") z5,
            out("zmm22") z6,
            out("zmm23") z7,
            out("zmm24") z8,
            out("zmm25") z9,
            out("zmm26") z10,
            out("zmm27") z11,
            out("zmm28") z12,
            out("zmm29") z13,
            out("zmm30") z14,
            out("zmm31") z15,
        );

        // TODO: how can we guarantee that vpext doesnt just use our own registers

        // VPEXT adds 70 cycles maximum.
        // VPEXT makes the LUT overrated
        // unless we have better heuristics its better to branch. branching is insanely cheap.

        //let entry_ndxs = black_box(vpext512(words, bitmasks, lookup_mask));

        /*
        bitmasks = _mm512_maskz_mov_epi32(!lookup_mask, bitmasks);
        expected = _mm512_maskz_mov_epi32(!lookup_mask, expected);
        // branches = _mm512_maskz_mov_epi32(lookup_mask, branches);

        // alright im writing slop, but the idea is this:
        // before loop:
        // ndxs + (padding to start of LUT)
        //
        // in loop:
        // mask = incrementing
        // each time we copy mask, and it with the original.
        // then we basically branches[mask] = entry_ndxs[mask] + base_zmm (using vpermw/d)
        //
        //        // For each one:
        // - If its in the mask, extract its ndx and shift to position
        // - same trick with incrementing mask is likely best, we just have to and it with the
        // original mask
        // - but there might be a problem with a
        arch::asm!(
            "mov {offset:e}, 1",
            "kmovw k2, {offset:e}",
            // This instruction is goated, but my logic is shit
            // note: not k2, but k2 & lookup_mask
            "vpshufd {temp}, , 0x02\n",
            "vpbroadcastd {temp}, "
            "vpermw {branches} {{k2}}",
            "kshiftlw k2, k2, 1",
            offset = out(reg) _,
            temp = out(reg) _,
            branches = in(zmm_reg) branches,
            z0 = in(zmm_reg) z0,

        );*/

        /*
        debug_zmm(bitmasks, "bitmasks");
        debug_zmm(expected, "ebitmasks");
        debug_zmm(branches, "bbitmasks");*/
    }
}*/

#[inline(always)]
pub unsafe fn build_branch(ndxs: __m512i, words: __m512i) -> Branch {
    // ndxs -> u32x16

    // TODO: if a branch is a zero, we can optimize it out with a mask register and stop going
    // TODO: mask pext, load all cache lines into zmm registers

    unsafe {
        let zero = _mm512_setzero_si512();
        //let one = _mm512_set1_epi32(1);

        let decoder_table = &_generated::ENTRIES as *const _ as *const i32;
        // Scale by 64.
        // TODO: watch out for scale
        let ndxs = _mm512_slli_epi32(ndxs, 3);

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
        let branch_mask: __mmask16 = _mm512_movepi32_mask(types);
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

        let entry_ndxs = vpext512(words, bitmasks, !branch_mask);

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
