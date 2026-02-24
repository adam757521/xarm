// Good luck debugging this shitshow
use crate::{fetcher, ir};

use std::io::Read;
use quick_xml::reader::Reader;
use quick_xml::events::Event;

/*
 * We save:
 * Docvar info: (ISA, Mnemonic, instr-class and all that stuff)
 * All instruction identifiers
 * Encodings: box diagrams, docvar overrides, operands
 *
 * Nice to have:
 * - Explanation, as comments
 */

#[derive(Debug, Default)]
struct Docvars {
    mnemonic: Option<Box<str>>,
    instr_class: Option<Box<str>>,
    isa: Option<Box<str>>,
}

fn parse_docvar(reader: &mut Reader<&[u8]>) -> Docvars {
    let mut docvars = Docvars::default();
    let mut buf = Vec::with_capacity(512);

    let mut docvars_found = false;

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let is_docvars_start = e.name().as_ref() == b"docvars";
                assert!(is_docvars_start == !docvars_found);
                if is_docvars_start {
                    docvars_found = true;
                    continue;
                } 

                assert!(e.name().as_ref() == b"docvar");
                let mut attrs = e.attributes().flatten();
                let key = attrs.find(|a| a.key.as_ref() == b"key").unwrap().value;
                let value = attrs.find(|a| a.key.as_ref() == b"value").unwrap().value;
                let boxed = Box::from(String::from_utf8_lossy(value.as_ref()));

                match key.as_ref() {
                    b"mnemonic" => {
                        docvars.mnemonic = Some(boxed);
                    },
                    b"instr-class" => {
                        docvars.instr_class = Some(boxed);
                    },
                    b"isa" => {
                        docvars.isa = Some(boxed);
                    }
                    _ => {}
                }
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"docvars" => break,
            Ok(Event::Eof) => panic!(),
            _ => {}
        }
    }

    docvars
}

#[derive(Debug, Copy, Clone)]
enum Bit {
    One,
    Zero,
    NotOne,
    NotZero,
    Variable,
    Inherited
}

impl From<u8> for Bit {
    fn from(value: u8) -> Self {
        match value {
            b'1' => Bit::One,
            b'0' => Bit::Zero,
            b'x' => Bit::Variable,
            b'N' => Bit::NotOne,
            b'Z' => Bit::NotZero,
            _ => { println!("{}", value as char); panic!() }
        }

    }
}

#[derive(Debug, Clone)]
struct BitBox {
    bits: Vec<Bit>,
    hibit: Option<usize>,
    name: Option<Box<str>>
}

fn parse_boxes(reader: &mut Reader<&[u8]>, start_box: Option<BitBox>) -> Vec<BitBox> {
    let mut res = vec![];
    let mut buf = Vec::with_capacity(512);

    let mut current_c = false;
    let mut current_box = start_box;

    loop {
        buf.clear();

        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"box" => {
                        let mut attrs = e.attributes().flatten();

                        let hibit = attrs
                            .find(|a| a.key.as_ref() == b"hibit")
                            .map(|v| std::str::from_utf8(v.value.as_ref()).unwrap().parse::<usize>().unwrap());

                        let name = attrs
                            .find(|a| a.key.as_ref() == b"name")
                            .map(|v| Box::from(String::from_utf8_lossy(v.value.as_ref())));

                        current_box = Some(BitBox {
                            bits: vec![],
                            name,
                            hibit
                        });
                    },
                    b"c" => {
                        current_c = true;
                    },
                    _ => {
                        break;
                    }
                }
            },
            Ok(Event::Text(ref e)) => {
                if current_c {
                    let boxed = e.decode().unwrap();
                    let c = boxed.as_ref();
                    let bit = if c.len() == 1 {
                        c.as_bytes()[0]
                    } else if c.len() == 3 {
                        c.as_bytes()[1]
                    } else {
                        if c.starts_with("!= ") {
                            for character in c[3..].chars() {
                                // TODO: this box is not the size of the NZN, is this fine?
                                let p = match character {
                                    '1' => Bit::NotOne,
                                    '0' => Bit::NotZero,
                                    ch => (ch as u8).into()
                                };
                                current_box.as_mut().unwrap().bits.push(p);
                            }
                            continue;
                        }

                        panic!("Unexpected length")
                    };

                    current_box.as_mut().unwrap().bits.push(bit.into());
                }
            }
            Ok(Event::Empty(ref e)) => {
                if e.name().as_ref() != b"c" {
                    continue;
                }

                let mut attrs = e.attributes().flatten();

                let colspan = attrs
                    .find(|a| a.key.as_ref() == b"colspan")
                    .map(|v| std::str::from_utf8(v.value.as_ref()).unwrap().parse::<usize>().unwrap());

                match colspan {
                    Some(v) => {
                        for _ in 0..v {
                            current_box.as_mut().unwrap().bits.push(Bit::Variable);
                        }
                    }
                    None => {
                        // TODO: at least i fucking think it is
                        current_box.as_mut().unwrap().bits.push(Bit::Inherited);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                if current_c { 
                    if e.name().as_ref() == b"c" {
                        current_c = false;
                    }
                    continue;
                }

                if e.name().as_ref() != b"box" {
                    break;
                }

                res.push(current_box.clone().unwrap());
            },
            Ok(Event::Eof) => panic!(),
            _ => {}
        }
    }

    res
}

#[derive(Debug)]
struct Encoding {
    name: Option<Box<str>>,
    docvar: Docvars,
    boxes: Vec<BitBox>
}

fn parse_encoding(reader: &mut Reader<&[u8]>, name: Option<Box<str>>) -> Encoding {
    let mut buf = Vec::new();

    let docvar = parse_docvar(reader);
    let mut boxes = vec![];

    loop {
        buf.clear();
        let e = reader.read_event_into(&mut buf);

        match e {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() != b"box" {
                    continue
                }

                let mut attrs = e.attributes().flatten();

                let hibit = attrs
                    .find(|a| a.key.as_ref() == b"hibit")
                    .map(|v| std::str::from_utf8(v.value.as_ref()).unwrap().parse::<usize>().unwrap());

                let name = attrs
                    .find(|a| a.key.as_ref() == b"name")
                    .map(|v| Box::from(String::from_utf8_lossy(v.value.as_ref())));

                boxes.extend(parse_boxes(reader, Some(BitBox {
                    bits: vec![],
                    name,
                    hibit
                })));

                break;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"encoding" => break,
            _ => {}
        }
    }

    Encoding {
        name,
        docvar,
        boxes
    }
}

#[derive(Debug)]
struct IClass {
    docvar: Docvars,
    base_boxes: Vec<BitBox>,
    encodings: Vec<Encoding>
}

fn parse_iclass(reader: &mut Reader<&[u8]>) -> IClass {
    let mut buf = Vec::new();

    let mut boxes = None;
    let mut encodings = Vec::new();
    let docvar = parse_docvar(reader);

    loop {
        buf.clear();
        let e = reader.read_event_into(&mut buf);
        match e {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                match e.name().as_ref() {
                    b"regdiagram" => {
                        boxes = Some(parse_boxes(reader, None));
                    }
                    b"encoding" => {
                        let mut attrs = e.attributes().flatten();

                        let name = attrs
                            .find(|a| a.key.as_ref() == b"name")
                            .map(|v| Box::from(String::from_utf8_lossy(v.value.as_ref())));

                        encodings.push(parse_encoding(reader, name));
                    }
                    _ => {}
                }
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"iclass" => break,
            Ok(Event::Eof) => panic!(),
            _ => {}
        }
    }

    IClass {
        docvar,
        base_boxes: boxes.unwrap(),
        encodings
    }
}

#[derive(Debug)]
struct Specification {
    global_docvar: Docvars,
    iclasses: Vec<IClass>
}

fn parse_spec(reader: &mut Reader<&[u8]>) -> Specification {
    let mut event_buf = Vec::new();

    let mut global_docvar = None;
    let mut iclasses = Vec::new();

    loop {
        event_buf.clear();
        match reader.read_event_into(&mut event_buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                b"instructionsection" => {
                    global_docvar = Some(parse_docvar(reader));
                },
                b"iclass" => {
                    iclasses.push(parse_iclass(reader));
                },
                _ => {}
                }
            }
            Ok(Event::Eof) => break,
            _ => (),
        }
    }

    Specification {
        global_docvar: global_docvar.unwrap(),
        iclasses
    }
}

fn iclass_into_ir(_global_docvar: &Docvars, iclass: &IClass) -> Vec<ir::Instruction> {
    // TODO: will be way better to make this array safer.
    let mut base_bit_pattern = [None; 32];
    let mut base_filter_ranges = Vec::new();

    for b in &iclass.base_boxes {
        let mut cndx = b.hibit.unwrap();
        let len = b.bits.len();
        let mut added = false;

        for bit in &b.bits {
            let mapped_bit = match bit {
                Bit::One => Some(ir::Bit::One),
                Bit::Zero => Some(ir::Bit::Zero),
                Bit::NotOne => {
                    if !added {
                        base_filter_ranges.push((b.hibit.unwrap()+1)-len..=b.hibit.unwrap());
                        added = true
                    }
                    Some(ir::Bit::NotOne)
                }
                Bit::NotZero => {
                    if !added {
                        base_filter_ranges.push((b.hibit.unwrap()+1)-len..=b.hibit.unwrap());
                        added = true
                    }
                    Some(ir::Bit::NotZero)
                }
                Bit::Variable | Bit::Inherited => {
                    if cndx != 0 { cndx -= 1; }
                    continue
                }
            };

            base_bit_pattern[cndx] = mapped_bit;

            // TODO what abut  names

            // Safety: if index is 0, there are no more bits.
            if cndx != 0 { cndx -= 1; }
        }
    }

    iclass
        .encodings
        .iter()
        .map(|e| {
            let mut bit_pattern = base_bit_pattern.clone();
            let mut filter_ranges = base_filter_ranges.clone();
            
            for b in &e.boxes {
                let mut cndx = b.hibit.unwrap();
                let len = b.bits.len();
                let mut added = false;
                
                for bit in &b.bits {
                    let mapped_bit = match bit {
                        Bit::One => Some(ir::Bit::One),
                        Bit::Zero => Some(ir::Bit::Zero),
                        Bit::NotOne => {
                            if !added {
                                filter_ranges.push((b.hibit.unwrap()+1)-len..=b.hibit.unwrap());
                                added = true;
                            }
                            Some(ir::Bit::NotOne)
                        }
                        Bit::NotZero => {
                            if !added {
                                filter_ranges.push((b.hibit.unwrap()+1)-len..=b.hibit.unwrap());
                                added = true;
                            }
                            Some(ir::Bit::NotZero)
                        }
                        Bit::Variable | Bit::Inherited => {
                            if cndx != 0 { cndx -= 1; }
                            continue
                        }
                    };
                    bit_pattern[cndx] = mapped_bit;

                    // Safety: if index is 0, there are no more bits.
                    if cndx != 0 { cndx -= 1; }
                }
            }
            /*

            if e.docvar.mnemonic == Some(Box::from("PLDW")) {
                println!("start {}", filter_ranges.len());
                println!("{:?}", iclass.base_boxes);
                println!("{:?}", e.boxes);
            }*/

            //if filter_ranges.len() == 0 {
            if true {
                return Some(ir::Instruction {
                filters: Box::from(filter_ranges),
                pattern: bit_pattern,
                regions: Box::new([]),
                name: e.name.clone().unwrap()
            }); } else {
                    return None;

            }
        })
    .filter_map(|x| x)
    .collect::<Vec<ir::Instruction>>()
}

// Maybe spec view.. just to inherit easily
fn specs_into_ir(specs: Vec<Specification>) -> Vec<ir::Instruction> {
    // This basically is classification of what we need, filtering.

    let mut instructions = vec![];

    for s in specs {
        for iclass in s.iclasses {
            // TODO: handle empty isa and instr_class better
            if iclass.docvar.isa == Some(Box::from("A32")) {
                if iclass.docvar.instr_class == Some(Box::from("general")) {
                    // We convert all encodings of this iclass into the IR.
                    instructions.extend(iclass_into_ir(&s.global_docvar, &iclass));
                }
            }
        }
    }

    instructions
}

pub fn parse_into_ir(mut iss: fetcher::arm::InstructionSpecificationStream) -> Vec<ir::Instruction> {
    let mut iterator = iss.make_iter().unwrap();

    let mut file_contents = Vec::with_capacity(64 * 4096);
    let mut specifications = Vec::with_capacity(2200);

    let ignore = vec![
        "shared_pseudocode.xml",
        "constraint_text_mappings.xml",
        "encodingindex.xml",
        "fpsimdindex.xml",
        "index.xml",
        "mortlachindex.xml",
        "notice.xml",
        "sveindex.xml",

        "a32_encindex.xml",
        "t32_encindex.xml",
    ];

    while let Some(mut entry) = iterator.next() {
        let path = entry.header().path().unwrap();
        if ignore.iter().any(|i| path.ends_with(i)) {
            continue;
        }
        // println!("{path:?}");

        file_contents.clear();

        entry.read_to_end(&mut file_contents).unwrap();

        let mut reader = Reader::from_reader(file_contents.as_slice());
        specifications.push(parse_spec(&mut reader));
    }

    specs_into_ir(specifications)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser() {
        return;
        const ARM_SPEC_32: &str = "https://developer.arm.com/-/cdn-downloads/permalink/Exploration-Tools-AArch32-ISA/ISA_AArch32/ISA_AArch32_xml_A_profile-2025-12.tar.gz";
        //const ARM_SPEC: &str = "https://developer.arm.com/-/cdn-downloads/permalink/Exploration-Tools-A64-ISA/ISA_A64/ISA_A64_xml_A_profile-2025-06.tar.gz";

        let stream = crate::fetcher::arm::InstructionSpecificationStream::connect(ARM_SPEC_32).unwrap();
        parse_into_ir(stream);
    }
}
