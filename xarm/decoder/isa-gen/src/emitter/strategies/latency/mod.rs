pub mod graph;
pub mod lut;
pub mod instruction;

use isa_gen_nostd::Descriptor;
use crate::emitter::traits::CodeEmitter;
use quote::quote;
use proc_macro2::TokenStream;

fn emit_use() -> TokenStream {
    quote! {
        use isa_gen_nostd::{Entry, DescriptorEntry, InnerData, BranchData, LookupData};
    }
}

fn const_desc(d: &Descriptor) -> TokenStream {
    match d {
        Descriptor::Branch(branch) => {
            let then_raw = branch.then.0;
            let else_raw = branch.r#else.0;
            let bbm = branch.bitmask;
            let bex = branch.expected;
            quote! { 
                Entry { 
                    tag: 0,
                    data: InnerData {
                        branch: BranchData {
                            bitmask: #bbm, 
                            expected: #bex, 
                            then: DescriptorEntry(#then_raw), 
                            r#else: DescriptorEntry(#else_raw) 
                        }
                    }
                }
            }
        }
        Descriptor::Lookup(lookup) => {
            let entry_raws = lookup.entries.iter().map(|e| e.0);
            let bm = lookup.bitmask;
            let hint = lookup._hint;
            quote! { 
                Entry { 
                    tag: 0x8000_0000,
                    data: InnerData {
                        lookup: LookupData {
                            bitmask: #bm, 
                            _hint: #hint,
                            entries: [#(DescriptorEntry(#entry_raws)),*] 
                        }
                    }
                } 
            }
        }
        // TODO: we have to handle empty descriptors better.
        // THIS IS GOING TO FUCKING BYTE ME IN THE ASS
        Descriptor::Empty => quote! { Entry { tag: 0, lookup: unsafe { std::mem::zeroed() } } },
    }
}

fn emit_descriptors(root_bit: u32, pool: Vec<Descriptor>, descriptors: Vec<Descriptor>) -> TokenStream {
    let pool_consts = pool.iter().map(|desc| const_desc(desc));
    let pool_len = pool.len();

    let l1_consts = descriptors.iter().map(|desc| const_desc(desc));
    let l1_len = descriptors.len();

    quote! {
        pub static DECODER_POOL: [Entry; #pool_len] = [
            #(#pool_consts),*
        ];

        pub static ROOT_DESCS: [Entry; #l1_len] = [
            #(#l1_consts),*
        ];

        pub static ROOT_BITMASK: u32 = #root_bit;
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
        let descriptors = emit_descriptors(b, pool, descriptors);
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
        return;
        const ARM_SPEC_32: &str = "https://developer.arm.com/-/cdn-downloads/permalink/Exploration-Tools-AArch32-ISA/ISA_AArch32/ISA_AArch32_xml_A_profile-2025-12.tar.gz";

        let stream = crate::fetcher::arm::InstructionSpecificationStream::connect(ARM_SPEC_32).unwrap();
        let instructions = crate::parser::arm::parse_into_ir(stream);

        let _ = super::graph::build(&instructions.iter().collect::<Vec<_>>());
        //let (b, pool, descriptors) = super::lut::build(entry_node);

    }
}
