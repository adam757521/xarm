use super::graph::Node;
use crate::ir;

use isa_gen_nostd::{Descriptor, DescriptorEntry};

pub fn add_to_l1(insts: &[&ir::Instruction], entry: &Node, desc_pool: &mut Vec<Descriptor>, l1: &mut Vec<Descriptor>) {
    // TODO: Probably no duplicates here, right
    match entry {
        Node::Lookup { entries, bits, .. } => {
            assert!(entries.len() <= 16);
            let mut mapped_entries = [DescriptorEntry(DescriptorEntry::TAG_NOT_PRESENT); 16];

            for (ndx, entry) in entries.iter().enumerate() {
                if let Some(entry) = entry {
                    mapped_entries[ndx] = add_entry_as_descriptor(insts, entry, desc_pool);
                }
            }

            l1.push(Descriptor::Lookup {
                bitmask: bits.iter().fold(0u32, |mask, &i| mask | (1 << i)),
                hint: 0,
                entries: mapped_entries
            });
        }
        Node::Branch { bitmask, value, then, r#else } => {
            l1.push(Descriptor::Branch {
                bitmask: *bitmask,
                expected: *value,
                then: add_entry_as_descriptor(insts, then, desc_pool),
                r#else: add_entry_as_descriptor(insts, r#else, desc_pool)
            });
        }
        _ => unreachable!()
    }
}

pub fn add_entry_as_descriptor(insts: &[&ir::Instruction], entry: &Node, descs_lut: &mut Vec<Descriptor>) -> DescriptorEntry {
    // TODO: Probably no duplicates here, right
    match entry {
        Node::Lookup { entries, bits, .. } => {
            let placeholder = descs_lut.len();
            let lookup_entry_descriptor = DescriptorEntry::new_lookup(placeholder as u16);
            descs_lut.push(Descriptor::Empty);

            assert!(entries.len() <= 16);
            let mut mapped_entries = [DescriptorEntry(DescriptorEntry::TAG_NOT_PRESENT); 16];

            for (ndx, entry) in entries.iter().enumerate() {
                if let Some(entry) = entry {
                    mapped_entries[ndx] = add_entry_as_descriptor(insts, entry, descs_lut);
                }
            }

            descs_lut[placeholder] = Descriptor::Lookup {
                bitmask: bits.iter().fold(0u32, |mask, &i| mask | (1 << i)),
                hint: 0,
                entries: mapped_entries
            };
            lookup_entry_descriptor
        }
        Node::Branch { bitmask, value, then, r#else } => {
            let placeholder = descs_lut.len();
            let branch_entry_descriptor = DescriptorEntry::new_branch(placeholder as u16);
            descs_lut.push(Descriptor::Empty);

            descs_lut[placeholder] = Descriptor::Branch {
                bitmask: *bitmask,
                expected: *value,
                then: add_entry_as_descriptor(insts, then, descs_lut),
                r#else: add_entry_as_descriptor(insts, r#else, descs_lut)
            };
            branch_entry_descriptor
        }
        Node::Leaf(inst) => {
            DescriptorEntry::new_leaf(insts.iter().position(|e| e == inst).unwrap() as u16)
        }
    }
}

pub fn build(instructions: &[&ir::Instruction], entry_node: Node) -> (u32, Vec<Descriptor>, Vec<Descriptor>) {
    let Node::Lookup { bits, entries, .. } = entry_node else { panic!() };

    // Scalar Optimization:
    // - Data Optimized: Great for random instructions, bad for a hot loop, slowest in worst case
    // - Speed Optimized: Insanely bad for random instructions, good for hot loop, faster in worst
    //
    // Branch prediction: Inevitable in both, but whatever
    // Hybrid: pooling, overhead for cache handling, hashing, commonly used, just ideas

    // Going for a data optimized approach.


    // TODO: branch, leaf, lookup
    // we need to essentially look up a table, and we have a 64-byte boundry to work with.
    // actually, if we can index into the middle of a line, itll be way better maybe (i dont think
    // we can even do that)
    // scrap that idea, we need a 64 byte value for all of these.

    // can optimize by not having a branch to leave but an unroll with ZII.
    // branch - simple embedding of bitmask, value....
    //
    // Wait, for the first level, we are guaranteed to never hit a Leaf.
    // We are only hitting a leaf on Branch or Lookup...

    // We essentially store it twice (identifiers), but at this point, this is fine. we have bunch
    // of headroom.

    let first_level_entries = entries;
    let first_level_iter = first_level_entries.into_iter().filter_map(|e| e).map(|e| *e);
    let mut first_level_descriptors = Vec::with_capacity(1usize << bits.len());

    let zero_desc_entry = DescriptorEntry::new_branch(0);
    let mut descriptor_pool = vec![Descriptor::Branch {
        bitmask: 0,
        expected: 0,
        then: zero_desc_entry,
        r#else: zero_desc_entry
    }];

    for entry in first_level_iter {
        assert!(!matches!(entry, Node::Leaf(_)));
        let _ = add_to_l1(instructions, &entry, &mut descriptor_pool, &mut first_level_descriptors);
        //first_level_descriptors.push(add_entry_as_descriptor(instructions, &entry, &mut descriptor_pool));
    }

    let mut bitmasks = vec![];

    for e in &first_level_descriptors {
        match e {
            Descriptor::Lookup { bitmask, .. } => {
                bitmasks.push(bitmask);
            }
            _ => {}
        };
    }

    for p in bitmasks {
        println!("{p:032b}");
    }
    // TODO: a few approaches, either precompute the worst case scenario in each level - or hard
    // code it.
    // if we precompute, where do we store? tf? another memory access?

    // we make two versions, go from left or right, and we just decide based on the calculated max.
    // we make a version which is always from right to eliminate branch.
    // we make a version which uses the level's precomputed, but this is only relevant for a
    //
    // pipeline which doesnt always feed and stops. (btw, even if it is precomputed, where do we
    // get it from, we might need a global register or something)
    println!("{}", descriptor_pool.len());

    // TODO: can definetly be optimized with "two ahead or three ahead" type branching.. can cut it
    // down DRAMATICALLY, we have a bunch of ...
    // TLB is another consideration, making a huge page for it can be nice would be nicer if we had
    // perfect hashing in that case
    // TODO: exploit ILP
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
