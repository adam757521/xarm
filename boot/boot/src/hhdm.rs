// TODO: Improve code quality.

use mm::pmm::bitset::BoolBitset;
use core::slice;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MemoryRange {
    pub pfn_start: u64,
    pub number_of_pages: usize
}

pub struct RAMChunks {
    pub chunks: [MemoryRange; 256],
    pub len: usize
}

static mut COALASCED_RAM_CHUNKS: RAMChunks = RAMChunks {
    chunks: [MemoryRange { pfn_start: 0, number_of_pages: 0}; 256],
    len: 0
};

// Safety: Single-threaded.
pub unsafe fn push_to_chunks(range: MemoryRange) -> Option<()> {
    unsafe { 
        if COALASCED_RAM_CHUNKS.len >= 256 {
            None
        } else {
            COALASCED_RAM_CHUNKS.chunks[COALASCED_RAM_CHUNKS.len] = range;
            COALASCED_RAM_CHUNKS.len += 1;

            Some(())
        }
    }
}

// I need functionality, of going through the bitset, with contigious windows
// with these windows, at the time of iteration, we map based on alignment and more windowing.
// but really, this should be the job of the bootstrap mapper, which is a bad abstraction currently
pub fn build_hhdm_ram(bitset: &BoolBitset) -> Option<&'static [MemoryRange]> {
    // I need the allocator for the mapper, and the allocator state changes, and that changes my
    // bitset at real time.
    // Solutions:
    // - Not a "windowed" real time approach, one process completes, then map it
    // problems:
    // need to have memory for the result itself, which can be arbitrary sized, or i can store it
    // in the bss like an idiot, which gives me a pretty valid way of having these new descriptors, 
    // but wastes memory, mapping it in heap would be even a bigger headache using UEFI because of
    // mmap key
    // then we can error with "RAM Too fragmented"
    //
    // - Copy the bitset, but that wastes even more memory
    //
    // - A streaming approach will work only with a temporary allocator, but that wastes memory
    // (and/or is a headache using free memory after its done)

    // Going with BSS.
    // NOTE: Obviously, insanely unoptimized.
    let mut l = 0;
    let mut r = 0;

    while r < bitset.len() {
        if bitset.get(l) {
            l += 1;
            r = l;
            continue;
        }

        if !bitset.get(r) {
            r += 1;
        } else {
            unsafe {
                push_to_chunks(MemoryRange {
                    pfn_start: l as u64,
                    number_of_pages: (r - l) as usize
                });
            }

            // end block
            l = r;
        }
    }

    if l < r {
        unsafe {
            push_to_chunks(MemoryRange {
                pfn_start: l as u64,
                number_of_pages: (r - l) as usize
            });
        }
    }

    Some(unsafe {
        slice::from_raw_parts(
            (&raw const COALASCED_RAM_CHUNKS.chunks) as *const MemoryRange,
            COALASCED_RAM_CHUNKS.len
        )
    })
}
