use crate::ir;

// Basically everything in here needs work.

pub fn categorization_differentiation() {
    todo!()
    // Less handlers, which are wider.
}

// Score how an individual bit "splits" the instructions into 50/50
pub fn score_split<'a>(bit_patterns: &[&'a [Option<ir::Bit>; 32]], bit: usize, score_var: bool) -> (f64, Vec<&'a [Option<ir::Bit>; 32]>, Vec<&'a [Option<ir::Bit>; 32]>) {
    // TODO: all these classification functions are huristic shits
    // TODO: need to take in mind all other states this is shit

    let mut counter_0: usize = 0;
    let mut counter_1: usize = 0;
    let mut counter_var: usize = 0;

    let mut res_0 = Vec::new();
    let mut res_1 = Vec::new();

    for bit_pattern in bit_patterns {
        match bit_pattern[bit] {
            Some(ir::Bit::One) => {
                counter_1 += 1;
                res_1.push(*bit_pattern);
            }
            Some(ir::Bit::Zero) => {
                counter_0 += 1;
                res_0.push(*bit_pattern);
            }
            None => counter_var += 1,
            _ => {}
        };
    }

    let pc0 = counter_0 as f64 / bit_patterns.len() as f64;
    let pc1 = counter_1 as f64 / bit_patterns.len() as f64;
    let pcv = counter_var as f64 / bit_patterns.len() as f64;

    let mut score = (pc0 - 0.5).abs() + (pc1 - 0.5).abs();
    if score_var {
        score += pcv * 2.0;
    }

    (score, res_0, res_1)
}

pub fn get_instruction_specialization<'a>(insts: &[&'a ir::Instruction]) -> Option<(u32, u32, Box<[&'a ir::Instruction]>, Box<[&'a ir::Instruction]>)> {
    // Used on instructions which can not be individually differentiated.
    // Ex.
    // 11110111X101XXXX1111XXXXXXX0XXXX: PLD
    // 11110111X101XXXX111100000110XXXX: PLD
    // Need to compare diff bitmask to specialization.

    let mut mask = 0u32;
    let mut value = 0u32;

    let mut then = vec![];
    let mut r#else = vec![];

    for col in 0..32 {
        let bits: Vec<_> = insts.iter().map(|i| i.pattern[col]).collect();

        let none = bits.iter().position(|b| b.is_none() || *b == Some(ir::Bit::NotZero) || *b == Some(ir::Bit::NotOne));
        let ones = bits.iter().position(|b| *b == Some(ir::Bit::One));
        let zeros = bits.iter().position(|b| *b == Some(ir::Bit::Zero));

        if none.is_none() {
            continue;
        }

        if ones.is_none() && zeros.is_none() {
            continue;
        }
        
        if ones.is_some() && zeros.is_some() {
            return None;
        }

        let inst_else = insts[none.unwrap()];
        let inst_then = if let Some(one) = ones {
            insts[one]
        } else {
            insts[zeros.unwrap()]
        };

        if !then.contains(&inst_then) {
            then.push(inst_then);
        }

        if !r#else.contains(&inst_else) {
            r#else.push(inst_else);
        }

        mask |= 1 << col;
        if ones.is_some() {
            value |= 1 << col;
        }
    }

    Some((mask, value, then.into(), r#else.into()))
}

pub fn can_individually_differentiate(bit_patterns: &[&[Option<ir::Bit>; 32]]) -> bool {
    (0..32).any(|col| {
        let mut bits = bit_patterns.iter().map(|p| match p[col] {
            Some(ir::Bit::NotZero) => None,
            Some(ir::Bit::NotOne) => None,
            None => None,
            other => other
        });
        
        let first = bits.next().flatten();
        match first {
            None => false,
            Some(f) => bits.all(|b| b.is_some()) && 
                       bit_patterns.iter().any(|p| p[col] != Some(f))
        }
    })
}

// This should prefer bits which are not X?
pub fn simple_individualistic_differentiation(bit_patterns: &[&[Option<ir::Bit>; 32]], budget: usize, penalize: bool) -> Vec<usize> {
    let mut selected_bits = Vec::with_capacity(budget);

    for _ in 0..budget {
        let mut best_bit = None;
        let mut best_total_score = f64::MAX;

        for i in 0..32 {
            if selected_bits.contains(&i) { continue; }

            let (score, _, _) = score_split(bit_patterns, i, penalize);

            if score < best_total_score {
                best_total_score = score;
                best_bit = Some(i);
            }
        }
        
        if let Some(b) = best_bit {
            selected_bits.push(b);
        }
    }

    selected_bits
}

/*
pub fn individualization_differentiation(bit_patterns: &[&[Option<ir::Bit>; 32]], budget: usize) -> Vec<usize> {
    // WTF
    let mut selected_bits = Vec::with_capacity(budget);
    let mut current_piles = vec![bit_patterns.to_vec()];

    for _ in 0..budget {
        let mut best_bit = None;
        let mut best_total_score = f64::MAX;

        for i in 0..32 {
            if selected_bits.contains(&i) { continue; }

            let mut bit_score = 0.0;
            for pile in &current_piles {
                let (score, _, _) = score_split(pile, i);
                bit_score += score;
            }

            if bit_score < best_total_score {
                best_total_score = bit_score;
                best_bit = Some(i);
            }
        }

        if let Some(bit) = best_bit {
            let mut next_piles = vec![];
            let mut helps = false;
            for pile in &current_piles {
                let (_, res_0, res_1) = score_split(&pile, bit);
                // maybe for all..
                if !(res_0.is_empty() && res_1.is_empty()) {
                    helps = true;
                    //return selected_bits;
                }

                if !res_0.is_empty() { next_piles.push(res_0); }
                if !res_1.is_empty() { next_piles.push(res_1); }
            }

            selected_bits.push(bit);
            current_piles = next_piles;
        } else {
            break;
        }
    }

    selected_bits
}*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_split() {
        let mut inst_1 = [None; 32];
        let mut inst_2 = [None; 32];
        inst_1[0] = Some(ir::Bit::One);
        inst_2[0] = Some(ir::Bit::Zero);

        let instructions = &[
            &inst_1,
            &inst_2,
        ];

        assert!(score_split(instructions, 0, false).0 == 0.);
        assert!(score_split(instructions, 1, false).0 == 1.);
    }

    /*
    #[test]
    fn test_individualization_hardcode() {
        let mut inst_1 = [None; 32];
        let mut inst_2 = [None; 32];
        inst_1[0] = Some(ir::Bit::One);
        inst_2[0] = Some(ir::Bit::Zero);
        inst_1[1] = Some(ir::Bit::One);
        inst_2[1] = Some(ir::Bit::One);

        let instructions = &[
            &inst_1,
            &inst_2,
        ];

        assert!(individualization_differentiation(instructions, 10) == vec![0]);
    }*/

    #[test]
    fn test_individualization() {
        return;
        const ARM_SPEC_32: &str = "https://developer.arm.com/-/cdn-downloads/permalink/Exploration-Tools-AArch32-ISA/ISA_AArch32/ISA_AArch32_xml_A_profile-2025-12.tar.gz";

        //const ARM_SPEC: &str = "https://developer.arm.com/-/cdn-downloads/permalink/Exploration-Tools-A64-ISA/ISA_A64/ISA_A64_xml_A_profile-2025-06.tar.gz";
        let stream = crate::fetcher::arm::InstructionSpecificationStream::connect(ARM_SPEC_32).unwrap();
        let instructions = crate::parser::arm::parse_into_ir(stream);
        println!("{}", instructions.len());

        let mapped: Vec<_> = instructions.iter().map(|e| &e.pattern).collect();
        println!("{}", mapped.len());

        fn generate_permutations(pattern: &str) -> Vec<String> {
            let mut results = Vec::new();
            expand(pattern.to_string(), 0, &mut results);
            results
        }

        fn expand(current: String, index: usize, results: &mut Vec<String>) {
            // Find the next 'X' starting from our current index
            if let Some(pos) = current[index..].find('X') {
                let x_pos = index + pos;

                // Try '0'
                let mut zero_branch = current.clone();
                zero_branch.replace_range(x_pos..x_pos+1, "0");
                expand(zero_branch, x_pos + 1, results);

                // Try '1'
                let mut one_branch = current;
                one_branch.replace_range(x_pos..x_pos+1, "1");
                expand(one_branch, x_pos + 1, results);
            } else {
                // No more 'X's left, we found a complete permutation
                results.push(current);
            }
        }

        fn get_key_for_pattern(pattern: &[Option<ir::Bit>; 32], bits: &[usize]) -> String {
            let mut result = String::new();

            for bit in bits {
                let b = pattern[*bit];
                let s = match b {
                    Some(ir::Bit::One) => "1",
                    Some(ir::Bit::Zero) => "0",
                    Some(ir::Bit::NotOne) => "N",
                    Some(ir::Bit::NotZero) => "Z",
                    None => "X"
                };
                result = result + s;
            }
            
            result
        }


        for i in 1..13 {
        let res = simple_individualistic_differentiation(mapped.as_slice(), i, false);
        println!("{res:?}");

        
        let mut mapa = std::collections::HashMap::<String, Vec<&[Option<ir::Bit>; 32]>>::new();
        for m in &mapped {
            for c in generate_permutations(&get_key_for_pattern(m, &res)) {
                mapa.entry(c).or_default().push(m);
            }
        }

        let sizes = mapa.iter().map(|(_, c)| c.len()).collect::<Vec<_>>();

        let min = sizes.iter().min().unwrap();
        let max = sizes.iter().max().unwrap();
        let sum: usize = sizes.iter().sum();
        let avg = sum as f64 / sizes.len() as f64;

        println!("Min: {}, Max: {}, Avg: {:.2}, Cnt: {}, Sum: {sum}", min, max, avg, mapa.len());
        
        }
        /*
        for (_, s) in mapa {
            break;
            if s.len() != 11 {
                continue;
            }

            let mut numbers = vec![];
            for i in 0..31 {
                numbers.push(i);
            }
            numbers.reverse();

            for c in s {
                println!("{:?}", get_key_for_pattern(c, &numbers));
            }
        }*/

        //panic!()
    }
}
