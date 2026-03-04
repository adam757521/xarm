// TODO: graph implies it connects to self
use crate::{
    ir,
    emitter::{classification, bits}
};

use std::collections::HashMap;
use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub enum Node<'a> {
    // TODO: having branch here means we have to branch predict in the hot path.
    Lookup {
        instructions: Box<[&'a ir::Instruction]>,
        bits: Box<[usize]>,
        entries: Box<[Option<Box<Node<'a>>>]>
    },
    Branch {
        bitmask: u32,
        value: u32,
        then: Box<Node<'a>>,
        r#else: Box<Node<'a>>,
    },
    Leaf(&'a ir::Instruction)
}

impl<'a> Node<'a> {
    /// Returns the maximum depth of the tree.
    /// A single Leaf has a depth of 1.
    pub fn max_depth(&self) -> usize {
        match self {
            Node::Leaf(_) => 1,
            Node::Branch { then, r#else, .. } => {
                1 + std::cmp::max(then.max_depth(), r#else.max_depth())
            }
            Node::Lookup { entries, .. } => {
                let sub_depth = entries
                    .iter()
                    .flatten() // Skip None entries
                    .map(|n| n.max_depth())
                    .max()
                    .unwrap_or(0);
                1 + sub_depth
            }
        }
    }

    /// Returns the average path length to reach a leaf.
    pub fn average_depth(&self) -> f64 {
        let (total_depth, count) = self.depth_sum_and_count(1);
        if count == 0 { 0.0 } else { total_depth as f64 / count as f64 }
    }

    /// Helper for average_depth: returns (sum of all leaf depths, count of leaves)
    pub fn depth_sum_and_count(&self, current_depth: usize) -> (usize, usize) {
        match self {
            Node::Leaf(_) => (current_depth, 1),
            Node::Branch { then, r#else, .. } => {
                let (s1, c1) = then.depth_sum_and_count(current_depth + 1);
                let (s2, c2) = r#else.depth_sum_and_count(current_depth + 1);
                (s1 + s2, c1 + c2)
            }
            Node::Lookup { entries, .. } => {
                entries
                    .iter()
                    .flatten()
                    .map(|n| n.depth_sum_and_count(current_depth + 1))
                    .fold((0, 0), |acc, res| (acc.0 + res.0, acc.1 + res.1))
            }
        }
    }
}

fn is_instruction_specialized(inst: &ir::Instruction, filter_range: RangeInclusive<usize>, bits: &[Option<ir::Bit>]) -> bool {
    inst.pattern[filter_range].iter().zip(bits).all(|(inst_bit, filter_bit)| {
        let filter_bit_mapped = match filter_bit {
            Some(ir::Bit::NotOne) => Some(ir::Bit::One),
            Some(ir::Bit::NotZero) => Some(ir::Bit::Zero),
            _ => None
        };

        if let Some(filter_bit_required) = filter_bit_mapped {
            inst_bit == &Some(filter_bit_required)
        } else {
            true
        }
    })
}

#[derive(Debug)]
struct SpecializationBranch<'a> {
    filter_range: RangeInclusive<usize>,
    filter_bits: &'a [Option<ir::Bit>],
    specialized: Vec<&'a ir::Instruction>,
    not_specialized: Vec<&'a ir::Instruction>
}

fn decide_specialization_branch<'a>(instructions: &[&'a ir::Instruction]) -> Option<SpecializationBranch<'a>> {
    let filter_cases = instructions
        .iter()
        .filter(|i| !i.filters.is_empty())
        .flat_map(|i| {
            i.filters.iter().map(|f| {
                (f, &i.pattern[f.clone()])
            })
        })
        // Use RangeInclusive as hash key to remove duplicates
        .collect::<HashMap<_, _>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut best_specialized_ratio: Option<f64> = None;
    let mut best_filter = None;
    for (range, bits) in &filter_cases {
        let mut specialized = vec![];
        let mut not_specialized = vec![];

        for inst in instructions {
            if is_instruction_specialized(inst, (*range).clone(), bits) {
                specialized.push(*inst);
            } else {
                not_specialized.push(*inst);
            }
        }

        let specialized_ratio = specialized.len() as f64 / instructions.len() as f64;
        if best_specialized_ratio.is_none() || ((specialized_ratio - 0.5).abs() < (best_specialized_ratio.unwrap() - 0.5).abs()) {
            best_specialized_ratio = Some(specialized_ratio);
            best_filter = Some(SpecializationBranch {
                filter_range: (*range).clone(),
                filter_bits: bits,
                specialized,
                not_specialized,
            });
        }
    }

    let specialized_range = 0.30..0.70;
    if let Some(best_ratio) = best_specialized_ratio {
        if specialized_range.contains(&best_ratio) {
            return best_filter;
        }
    }

    None
}

fn individualize_prefer_branch<'a>(instructions: &[&'a ir::Instruction], budget: usize) -> Node<'a> {
    let debugging = 
        instructions.iter().find(|i| i.name == Box::from("PLDW_i_A1")).is_some() &&
        instructions.iter().find(|i| i.name == Box::from("TST_i_A1")).is_some();


    if instructions.len() == 0 {
        panic!();
    }

    // TODO: multiple branches in one embedding?
    if instructions.len() == 1 {
        return Node::Leaf(instructions[0]);
    }

    // Prefer doing a branch when a filter is specialized to split evenly
    // Prefer ...            when instruction has a bit override, feels like cheap man filter
    // Fallback to differentiation

    // TODO: might have prob here
    if let Some(branch_decision) = decide_specialization_branch(instructions) {
        let specialization_bitmask = branch_decision.filter_bits.iter().map(|filter_bit| {
            match filter_bit {
                Some(ir::Bit::NotOne) | Some(ir::Bit::NotZero) => "1",
                _ => "0"
            }
        }).collect::<Vec<_>>().concat();

        let specialization_value = branch_decision.filter_bits.iter().map(|filter_bit| {
            match filter_bit {
                Some(ir::Bit::NotOne) => "1",
                _ => "0"
            }
        }).collect::<Vec<_>>().concat();

        let start = branch_decision.filter_range.start();

        let bitmask = u32::from_str_radix(&specialization_bitmask, 2).unwrap() << start;
        let value = u32::from_str_radix(&specialization_value, 2).unwrap() << start;

        return Node::Branch {
            bitmask,
            value,
            then: Box::new(individualize_prefer_branch(&branch_decision.specialized, 4)),
            r#else: Box::new(individualize_prefer_branch(&branch_decision.not_specialized, 4))
        };
    }

    let owned_patterns = instructions.iter().map(|i| &i.pattern).collect::<Vec<_>>();
    if !classification::can_individually_differentiate(&owned_patterns) {
        //pretty_print_bucket(instructions);
        if instructions.len() == 2 {
            /* Hardcode the LDR/LDRT case.
            NNNN010XX0X11111XXXXXXXXXXXXXXXX: LDR_l_A1
            NNNN0100X011XXXXXXXXXXXXXXXXXXXX: LDRT_A1
            */

            // TODO: solve this problem
            let bugged_pairs = vec![
                ("LDRSH_l_A1", "LDRSHT_A1"),
                ("LDRSB_l_A1", "LDRSBT_A1"),
                ("LDRH_l_A1", "LDRHT_A1"),
                ("LDR_l_A1", "LDRT_A1"),
                ("LDRB_l_A1", "LDRBT_A1"),
            ];

            // I think maybe P and W bits in docs, idk
            for p in bugged_pairs {
                if instructions.iter().find(|i| i.name == Box::from(p.0)).is_some() && instructions.iter().find(|i| i.name == Box::from(p.1)).is_some() {
                    return Node::Leaf(instructions[0]);
                }
            }

            if instructions[0].pattern == instructions[1].pattern {
                // TODO: handle alises better lol, or disble them
                return Node::Leaf(instructions[0]);
            }
        }

        let specialization = classification::get_instruction_specialization(instructions);
        if let Some((bitmask, value, then, r#else)) = specialization {
            return Node::Branch {
                bitmask,
                value,
                then: Box::new(individualize_prefer_branch(&then, 4)),
                r#else: Box::new(individualize_prefer_branch(&r#else, 4))
            };
        }

        //pretty_print_bucket(instructions);
        //println!("{:?}", instructions);

    }

    let (b, mapping) = bits::min_bits_for_individualisation(instructions, budget);

    let mut entries_mapping = vec![None; 1usize << b.len()];
    for (binary, bucket) in mapping {
        let index = usize::from_str_radix(&binary, 2).unwrap();
        let insts = bucket.into_iter().collect::<Vec<_>>();

        if debugging {
            /*
             *2
Printing bucket of 2
NNNN00111001XXXXXXXXXXXXXXXXXXXX: ORRS_i_A1
NNNN00110001XXXX0000XXXXXXXXXXXX: TST_i_A1

            */

        }

        entries_mapping[index] = Some(Box::new(individualize_prefer_branch(&insts, budget)));
    }

    Node::Lookup {
        bits: Box::from(b),
        instructions: Box::from(instructions),
        entries: Box::from(entries_mapping)
    }
}

fn pretty_print_bucket(bucket: &[&ir::Instruction]) {
    println!("Printing bucket of {}", bucket.len());
    let mut numbers = vec![];
    for i in 0..32 {
        numbers.push(i);
    }
    numbers.reverse();
    for i in bucket {
       println!("{}: {}", bits::get_key_for_pattern_nz(&i.pattern, &numbers), i.name);
    }
}

pub fn build<'a>(instructions: &[&'a ir::Instruction]) -> Node<'a> {
    let patterns: Vec<_> = instructions.iter().map(|i| &i.pattern).collect();
    let mut b = classification::simple_individualistic_differentiation(&patterns, 4, false);
    b.sort_by(|a, b| b.cmp(a));

    let mut entries_mapping = vec![None; 1usize << b.len()];
    for (binary, bucket) in bits::create_bit_mapping(instructions, &b) {
        let index = usize::from_str_radix(&binary, 2).unwrap();
        let bucket_insts = &bucket.into_iter().collect::<Vec<_>>();
        entries_mapping[index] = Some(Box::new(individualize_prefer_branch(bucket_insts, 4)));
    }

    Node::Lookup {
        bits: b.into(),
        instructions: Box::from(instructions),
        entries: entries_mapping.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits_to_bitmask(bits: &[usize]) -> u32 {
        bits.iter().fold(0u32, |mask, &i| mask | (1 << i))
    }

    fn walk_tree(node: &Node, word: u32) -> ir::Instruction {
        match node {
            Node::Lookup { bits, entries, .. } => {
                let bitmask = bits_to_bitmask(&bits);

                let index = unsafe {
                    core::arch::x86_64::_pext_u32(word, bitmask)
                };

                //println!("{index:032b}");
                //println!("{word:032b}");
                //println!("going through a bucket, {index:?} ");

                //pretty_print_bucket(instructions);
                
                walk_tree(&entries[index as usize].clone().unwrap(), word)
            },
            Node::Branch { bitmask, value, then, r#else } => {
                if word & *bitmask == *value {
                    println!("taking then branch");
                    walk_tree(then, word)
                } else {
                    println!("taking else branch");
                    walk_tree(r#else, word)
                }
            }
            Node::Leaf(result) => (*result).clone()
        }
    }

    #[test]
    fn test_graph() {
        // TODO: need pext (BMI2) to test
        const ARM_SPEC_32: &str = "https://developer.arm.com/-/cdn-downloads/permalink/Exploration-Tools-AArch32-ISA/ISA_AArch32/ISA_AArch32_xml_A_profile-2025-12.tar.gz";

        let stream = crate::fetcher::arm::InstructionSpecificationStream::connect(ARM_SPEC_32).unwrap();
        let instructions = crate::parser::arm::parse_into_ir(stream);

        // HVC_A1, STRH_i_A1_off
        //let instruction_word = 0x7C1F003F;
        // SBC/SBCS
        let instruction_word = 0b00000010110100000000000000000000;
        // PLDW_i_A1, SETPAN_A1
        //let instruction_word = 0b11110101000101011111000000000000;
        //let instruction_word = 0xD503201F;

        //let nop_instruction_word = 0b00000011001000001111000000000000;
        //let mov_i_instruction_word = 0b00000011101000000000000000000000;
        //let instruction_word = 0xE1600010;
        let entry = build(&instructions.iter().collect::<Vec<_>>());

        pretty_print_bucket(&[&walk_tree(&entry, instruction_word)]);
        println!("Instruction: {}", walk_tree(&entry, instruction_word).name);
        //println!("{instruction_word:032b}");
        //println!("{:032b}", instruction_word);
        //dbg!(entry_node);
        //panic!()
    }
}
