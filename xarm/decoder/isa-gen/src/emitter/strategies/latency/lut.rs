use super::graph::Node;
use crate::ir;

use isa_gen_nostd::{Descriptor, Entry};

pub fn add_to_l1(insts: &[&ir::Instruction], entry: &Node, desc_pool: &mut Vec<Entry>, l1: &mut Vec<Entry>) {
    // TODO: Probably no duplicates here, right
    match entry {
        Node::Lookup { entries, bits, .. } => {

            assert!(bits.len() <= 4);
            assert!(entries.len() <= 16);
            let mut mapped_entries = [Descriptor::new_invalid(); 16];

            for (ndx, entry) in entries.iter().enumerate() {
                if let Some(entry) = entry {
                    mapped_entries[ndx] = add_entry_as_descriptor(insts, entry, desc_pool);
                }
            }

            let mut bitmasks = [0; 4];
            for i in 0..bits.len() {
                bitmasks[i] = (1 << bits[i]) as u32;
            }

            l1.push(Entry {
                bitmasks,
                expected: bitmasks,
                entries: mapped_entries
            });
        }
        Node::Branch { bitmask, value, then, r#else } => {
            let mut descriptors = [Descriptor::new_invalid(); 16];
            descriptors[0] = add_entry_as_descriptor(insts, r#else, desc_pool);
            descriptors[0] = add_entry_as_descriptor(insts, then, desc_pool);

            l1.push(Entry {
                bitmasks: [*bitmask, 0, 0, 0],
                expected: [*value, 0, 0, 0],
                entries: descriptors
            });
        }
        _ => unreachable!()
    }
}

pub fn add_entry_as_descriptor(insts: &[&ir::Instruction], entry: &Node, descs_lut: &mut Vec<Entry>) -> Descriptor {
    // TODO: Probably no duplicates here, right?
    match entry {
        Node::Lookup { entries, bits, .. } => {
            assert!(bits.len() <= 4);
            let placeholder = descs_lut.len();
            let lookup_entry_descriptor = Descriptor::new_entry(placeholder as u16);
            descs_lut.push(Entry::default());

            assert!(entries.len() <= 16);
            let mut mapped_entries = [Descriptor::new_invalid(); 16];

            for (ndx, entry) in entries.iter().enumerate() {
                if let Some(entry) = entry {
                    mapped_entries[ndx] = add_entry_as_descriptor(insts, entry, descs_lut);
                }
            }

            let mut bitmasks = [0; 4];
            for i in 0..bits.len() {
                bitmasks[i] = (1 << bits[i]) as u32;
            }

            descs_lut[placeholder] = Entry {
                bitmasks,
                expected: bitmasks,
                entries: mapped_entries
            };
            lookup_entry_descriptor
        }
        Node::Branch { bitmask, value, then, r#else } => {
            let placeholder = descs_lut.len();
            let branch_entry_descriptor = Descriptor::new_entry(placeholder as u16);
            descs_lut.push(Entry::default());

            let mut descriptors = [Descriptor::new_invalid(); 16];
            descriptors[0] = add_entry_as_descriptor(insts, r#else, descs_lut);
            descriptors[0] = add_entry_as_descriptor(insts, then, descs_lut);
            descs_lut[placeholder] = Entry {
                bitmasks: [*bitmask, 0, 0, 0],
                expected: [*value, 0, 0, 0],
                entries: descriptors
            };
            branch_entry_descriptor
        }
        Node::Leaf(inst) => {
            // TODO: this should be better.. incase we would want to identify branch easily.
            Descriptor::new_leaf(insts.iter().position(|e| e == inst).unwrap() as u16)
        }
    }
}

pub fn build(instructions: &[&ir::Instruction], entry_node: Node) -> (u32, Vec<Entry>, Vec<Entry>) {
    let Node::Lookup { bits, entries, .. } = entry_node else { panic!() };

    // Scalar Optimization:
    // - Data Optimized: Great for random instructions, bad for a hot loop, slowest in worst case
    // - Speed Optimized: Insanely bad for random instructions, good for hot loop, faster in worst
    //
    // Hybrid: pooling, overhead for cache handling, hashing, commonly used, just ideas

    let first_level_entries = entries;
    let first_level_iter = first_level_entries.into_iter().filter_map(|e| e).map(|e| *e);

    let mut first_level_descriptors = Vec::with_capacity(1usize << bits.len());

    let mut descriptor_pool = vec![];

    for entry in first_level_iter {
        assert!(!matches!(entry, Node::Leaf(_)));
        let _ = add_to_l1(instructions, &entry, &mut descriptor_pool, &mut first_level_descriptors);
    }

    // TLB is another consideration, making a huge page for it can be nice would be nicer if we had
    // perfect hashing in that case
    (bits.iter().fold(0u32, |mask, &i| mask | (1 << i)), descriptor_pool, first_level_descriptors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        const ARM_SPEC_32: &str = "https://developer.arm.com/-/cdn-downloads/permalink/Exploration-Tools-AArch32-ISA/ISA_AArch32/ISA_AArch32_xml_A_profile-2025-12.tar.gz";

        let stream = crate::fetcher::arm::InstructionSpecificationStream::connect(ARM_SPEC_32).unwrap();
        let instructions = crate::parser::arm::parse_into_ir(stream);

        let r = instructions.iter().collect::<Vec<_>>();
        let entry_node = super::super::graph::build(&r);
        build(&r, entry_node);
    }
}
