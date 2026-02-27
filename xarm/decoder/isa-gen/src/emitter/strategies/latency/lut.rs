use super::graph::Node;
use crate::ir;

use isa_gen_nostd::{Descriptor, Entry};

pub fn add_entry_as_descriptor(insts: &[&ir::Instruction], entry: &Node, descs_lut: &mut Vec<Entry>) -> Descriptor {
    // TODO: Probably no duplicates here, right?
    match entry {
        Node::Lookup { entries, bits, .. } => {
            assert!(bits.len() <= 4);
            assert!(entries.len() <= 16);

            let placeholder = descs_lut.len();
            let lookup_entry_descriptor = Descriptor::new_entry(placeholder as u16);
            descs_lut.push(Entry::default());

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

            let expected = bitmasks.map(|mask| if mask != 0 { mask } else { 1 });

            descs_lut[placeholder] = Entry {
                bitmasks,
                expected,
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
                expected: [*value, 1, 1, 1],
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

pub fn build(instructions: &[&ir::Instruction], entry_node: Node) -> Vec<Entry> {
    let Node::Lookup { bits, entries, .. } = entry_node else { panic!() };

    // Scalar Optimization:
    // - Data Optimized: Great for random instructions, bad for a hot loop, slowest in worst case
    // - Speed Optimized: Insanely bad for random instructions, good for hot loop, faster in worst
    //
    // Hybrid: pooling, overhead for cache handling, hashing, commonly used, just ideas

    let first_level_entries = entries;
    let first_level_iter = first_level_entries.into_iter().filter_map(|e| e).map(|e| *e);

    let mut entry_pool = vec![];

    let mut first_level_descriptors = Vec::with_capacity(1usize << bits.len());
    for entry in first_level_iter {
        assert!(!matches!(entry, Node::Leaf(_)));
        first_level_descriptors.push(add_entry_as_descriptor(instructions, &entry, &mut entry_pool));
    }

    // TLB is another consideration, making a huge page for it can be nice would be nicer if we had
    // perfect hashing in that case
    let mut bitmasks = [0; 4];
    for i in 0..bits.len() {
        bitmasks[i] = (1 << bits[i]) as u32;
    }

    let expected = bitmasks.map(|mask| if mask != 0 { mask } else { 1 });

    entry_pool.insert(0, Entry {
        bitmasks,
        expected,
        entries: first_level_descriptors.try_into().unwrap()
    });
    entry_pool
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
