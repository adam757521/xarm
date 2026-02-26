pub mod graph;
pub mod lut;
pub mod instruction;

use isa_gen_nostd::Entry;
use crate::emitter::traits::CodeEmitter;
use quote::quote;
use proc_macro2::TokenStream;

fn emit_use() -> TokenStream {
    quote! {
        use isa_gen_nostd::{Entry, Descriptor};
    }
}

fn const_entry(e: &Entry) -> TokenStream {
    let entry_desc_raws = e.entries.iter().map(|e| e.0);
    let ee = e.expected;
    let eb = e.bitmasks;

    quote! {
        Entry {
            bitmasks: [#(#eb),*],
            expected: [#(#ee),*],
            entries: [#(Descriptor(#entry_desc_raws)),*]
        }
    }
}

fn emit_entries(l1_bit: u32, pool: Vec<Entry>, l1: Vec<Entry>) -> TokenStream {
    let pool_consts = pool.iter().map(|e| const_entry(e));
    let pool_len = pool.len();

    let l1_consts = l1.iter().map(|e| const_entry(e));
    let l1_len = l1.len();

    quote! {
        pub static ENTRIES: [Entry; #pool_len] = [
            #(#pool_consts),*
        ];

        pub static ROOT_ENTRIES: [Entry; #l1_len] = [
            #(#l1_consts),*
        ];

        pub static ROOT_BITMASK: u32 = #l1_bit;
    }
}

pub struct LatencyOptimizedCodeEmitter {

}

impl CodeEmitter for LatencyOptimizedCodeEmitter {
    fn emit() -> TokenStream {
        const ARM_SPEC_32: &str = "https://developer.arm.com/-/cdn-downloads/permalink/Exploration-Tools-AArch32-ISA/ISA_AArch32/ISA_AArch32_xml_A_profile-2025-12.tar.gz";

        let stream = crate::fetcher::arm::InstructionSpecificationStream::connect(ARM_SPEC_32).unwrap();
        let instructions = crate::parser::arm::parse_into_ir(stream);

        let patterns = instructions.iter().collect::<Vec<_>>();
        let entry_node = graph::build(&patterns);
        let (b, pool, descriptors) = lut::build(&patterns, entry_node);

        let usage = emit_use();
        let inst_enum = instruction::emit(&patterns);
        let descriptors = emit_entries(b, pool, descriptors);
        dbg!(descriptors.to_string());
        quote! {
            #usage

            #inst_enum

            #descriptors
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        const ARM_SPEC_32: &str = "https://developer.arm.com/-/cdn-downloads/permalink/Exploration-Tools-AArch32-ISA/ISA_AArch32/ISA_AArch32_xml_A_profile-2025-12.tar.gz";

        let stream = crate::fetcher::arm::InstructionSpecificationStream::connect(ARM_SPEC_32).unwrap();
        let instructions = crate::parser::arm::parse_into_ir(stream);

        let _ = super::graph::build(&instructions.iter().collect::<Vec<_>>());
        //let (b, pool, descriptors) = super::lut::build(entry_node);

    }
}
