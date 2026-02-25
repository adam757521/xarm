#![feature(target_feature_inline_always)]
#![feature(portable_simd)]

use core::hint::black_box;
use decoder::*;

/*
#[inline(always)]
#[target_feature(enable = "bmi2")]
pub unsafe fn test(word: u32) -> isa_gen_nostd::DescriptorEntry {
    let root_idx = core::arch::x86_64::_pext_u32(word, decoder::t::ROOT_BITMASK);
    let root_entry = *unsafe {
        decoder::t::ROOT_TABLE.get_unchecked(root_idx as usize)
    };

    black_box(root_entry)
}*/

use std::arch::x86_64::*;

#[inline(always)]
unsafe fn test_simd(base_ptr: *const i64, indices: __m512i) {
            //&decoder::t::ROOT_DESCS as *const _ as *const u64,
    unsafe { _mm512_i64gather_epi64::<8>(indices, base_ptr) };

}

#[target_feature(enable = "avx512cd")]
#[target_feature(enable = "avx512dq")]
#[target_feature(enable = "avx512vbmi")]
#[target_feature(enable = "avx512f")]
#[target_feature(enable = "bmi2")]
fn start_test() {
    let word: [u16; 8] = [
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,

    ];

    //let x = _mm512_setzero_si512();
    //let x = _mm512_set1_epi32(2);
    let x_512 = _mm512_set1_epi32(0x40 * 1);
    let words_512 = _mm512_set1_epi32(1123123);
    //let words_512 = black_box(_mm512_set1_epi32(0));
    //0b00001000
    //let masks_512 = _mm512_set1_epi32(0b110000000000000000000000);

    //let x_256 = _mm256_set1_epi32(2);
    //let masks_256 = _mm256_set1_epi32(0b00);

    //let res = unsafe { vpext512(x_512, masks_512) };
    //unsafe { debug_ymm(find_lead_bit(masks), "x") };
    //unsafe { debug_zmm(res, "x"); };
    //unsafe { build_branch_zmms(x) };

    //black_box(unsafe { playground(x_512, masks_512) } );

    /*
    unsafe {
        core::arch::asm!(
            "vpslld {ndx_zmm}, {ndx_zmm}, 6",
            ndx_zmm = in(zmm_reg) x_512,
        );
    }*/

    /*
    black_box(unsafe { playground(x_512, words_512) });
    black_box(unsafe { playground(x_512, words_512) });
    black_box(unsafe { playground(x_512, words_512) });
    black_box(unsafe { playground(x_512, words_512) });
    black_box(unsafe { playground(x_512, words_512) });
    black_box(unsafe { playground(x_512, words_512) });*/

    let start = std::time::Instant::now();
    let start_cycles = unsafe { std::arch::x86_64::_rdtsc() };

    let iterations: usize = 01_000_000_000 / 16;
    for _ in 0..iterations {

        black_box(unsafe { playground(x_512, words_512) });
        //black_box(unsafe { vpext512(words_512, x_512, 0x00FF) } );
        //black_box(unsafe { build_branch(x_512, words_512) });

        //black_box(unsafe { build_branch_zmms_8(x) });
        //core::hint::black_box(unsafe { hotpath_decode_3(x) });
        //black_box(test_simd(v));
        //black_box(unsafe { decode(word[i%2]); });


        //black_box(unsafe { test(word[i % 2]) });
        //black_box(unsafe { hotpath_decode(word) });
        //println!("h");
    }

    let end_cycles = unsafe { std::arch::x86_64::_rdtsc() } - start_cycles;
    let duration = start.elapsed();
    println!("{duration:?}. {end_cycles}");
    println!("c/i: {}", end_cycles as f64 / iterations as f64);

}

fn main() {
    /*
    let word = [
        0b11100001010000000000000001110000,
        0b11000001010000000000000001110000,
    ];*/

    unsafe { start_test() };

}
