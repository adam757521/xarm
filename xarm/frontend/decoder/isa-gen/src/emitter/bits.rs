use crate::ir;
use std::collections::{HashSet, HashMap};

pub fn get_key_for_pattern_nz(pattern: &ir::BitPattern, bits: &[usize]) -> String {
    bits.iter().map(|b| match pattern[*b] {
        Some(ir::Bit::One) => "1",
        Some(ir::Bit::Zero) => "0",
        Some(ir::Bit::NotOne) => "N",
        Some(ir::Bit::NotZero) => "Z",
        _ => "X"
    }).collect::<Vec<_>>().concat()
}

fn get_key_for_pattern(pattern: &ir::BitPattern, bits: &[usize]) -> String {
    bits.iter().map(|b| match pattern[*b] {
        Some(ir::Bit::One) => "1",
        Some(ir::Bit::Zero) => "0",
        _ => "X"
    }).collect::<Vec<_>>().concat()
}

fn expand(current: String, index: usize, results: &mut Vec<String>) {
    match current[index..].find('X') {
        Some(pos) => {
            let x_pos = index + pos;

            let mut zero_branch = current.clone();
            zero_branch.replace_range(x_pos..x_pos+1, "0");
            expand(zero_branch, x_pos + 1, results);

            let mut one_branch = current;
            one_branch.replace_range(x_pos..x_pos+1, "1");
            expand(one_branch, x_pos + 1, results);
        },
        None => results.push(current)
    }
}

pub fn get_all_bit_options(instruction: &ir::Instruction, bits: &[usize]) -> Vec<String> {
    let present_filter_cases = instruction
        .filters
        .iter()
        .filter(|range| (*range).clone().all(|bit| bits.contains(&bit)));

    let key = get_key_for_pattern(&instruction.pattern, bits);

    let mut results = Vec::new();
    expand(key, 0, &mut results);
    
    results
        .into_iter()
        .filter(|res| {
            let passes_filter = |string: String, filter_range: std::ops::RangeInclusive<usize>| {
                let string_filter_range = filter_range
                    .clone()
                    .map(|i| string.chars().nth(bits.iter().position(|b| b == &i).unwrap()).unwrap())
                    .collect::<String>();

                let pattern_key = filter_range
                    .map(|b| match instruction.pattern[b] {
                        Some(ir::Bit::NotZero) => '0',
                        Some(ir::Bit::NotOne) => '1',
                        None => 'X',
                        _ => unreachable!()
                    })
                    .collect::<String>();

                let mut blacklisted = Vec::new();
                expand(pattern_key.clone(), 0, &mut blacklisted);

                // TODO: this should be tested better
                !blacklisted.contains(&string_filter_range)
            };
            present_filter_cases.clone().all(|filter| passes_filter(res.clone(), filter.clone()))
        })
        .collect::<Vec<_>>()
}

pub fn create_bit_mapping<'a>(instructions: &[&'a ir::Instruction], bits: &[usize]) -> HashMap<String, HashSet<&'a ir::Instruction>> {
    let mut result: HashMap<_, HashSet<_>> = HashMap::new();
    
    for inst in instructions {
        for option in get_all_bit_options(inst, bits) {
            result.entry(option).or_default().insert(*inst);
        }

    }

    result
}

pub fn min_bits_for_individualisation<'a>(instructions: &[&'a ir::Instruction], budget: usize) -> (Vec<usize>, HashMap<String, HashSet<&'a ir::Instruction>>) {
    // TODO: make it simple for start, but it needs a heuristics aswell likely
    // TODO: this is shitty, and hella slow, but this can work
    
    let patterns = instructions.iter().map(|i| &i.pattern).collect::<Vec<_>>();

    let mut last_bits = crate::emitter::classification::simple_individualistic_differentiation(&patterns, 1, true);
    last_bits.sort_by(|a, b| b.cmp(a));
    let mut last_mapping = create_bit_mapping(instructions, &last_bits);
    let mut last_max = last_mapping.iter().map(|(_, c)| c.len()).max().unwrap();

    for bit_count in 2..=budget {
        let mut bits = crate::emitter::classification::simple_individualistic_differentiation(&patterns, bit_count, true);
        bits.sort_by(|a, b| b.cmp(a));
        let mapping = create_bit_mapping(instructions, &bits);
        let max = mapping.iter().map(|(_, c)| c.len()).max().unwrap();

        if max < last_max {
            last_max = max;
            last_mapping = mapping;
            last_bits = bits;
        } else {
            //return (last_bits, last_mapping);
        }
   }

    return (last_bits, last_mapping);
}
