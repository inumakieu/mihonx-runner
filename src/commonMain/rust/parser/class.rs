use super::uleb::read_uleb128;
use crate::{
    parser::strings::parse_string_at_offset, types::{
        Class_Def_Item, CodeItem, DexClass, DexContainer, DexField, DexMethod, DexValue,
        Header_Item, Instruction, NO_INDEX,
    }, utils::{parse_i16, parse_i32, parse_u16, parse_u32, parse_u64, get_lower_bits}
};
use std::{collections::HashMap, string};

fn parse_encoded_array(
    data: &[u8],
    offset: usize,
    container: &DexContainer,
) -> (Vec<DexValue>, usize) {
    let mut cursor = (offset as usize)
        .checked_sub(container.header_item.data_off as usize)
        .expect("String offset is before data section");
    let (size, c) = read_uleb128(data, cursor);
    cursor = c;

    let mut values = Vec::new();
    for _ in 0..size {
        let (val, new_cursor) = parse_encoded_value(data, cursor, container);
        values.push(val);
        cursor = new_cursor;
    }
    (values, cursor)
}

fn parse_encoded_value(data: &[u8], offset: usize, container: &DexContainer) -> (DexValue, usize) {
    let mut cursor = offset;
    let byte = data[cursor];
    cursor += 1;

    let val_type = byte & 0x1f; // lower 5 bits
    let val_arg = (byte >> 5) & 0x07; // upper 3 bits
    let size = (val_arg as usize) + 1; // actual byte length

    // println!("val_type = 0x{:x}, val_arg = 0x{:x}, size = {}", val_type, val_arg, size);

    match val_type {
        0x00 => {
            // VALUE_BYTE
            let v = data[cursor] as i8 as i32;
            cursor += 1;
            (DexValue::Int(v), cursor)
        }
        0x02 => {
            // VALUE_SHORT
            let mut buf = [0u8; 2];
            for i in 0..size {
                buf[i] = data[cursor + i];
            }
            let v = i16::from_le_bytes(buf) as i32;
            cursor += size;
            (DexValue::Int(v), cursor)
        }
        0x03 => {
            // VALUE_CHAR
            let mut buf = [0u8; 2];
            for i in 0..size {
                // println!("size = {} i = {}", size, i);
                buf[i] = data[cursor + i];
            }
            let v = u16::from_le_bytes(buf);
            cursor += size;
            (DexValue::Char(v), cursor)
        }
        0x04 => {
            // VALUE_INT
            let mut buf = [0u8; 4];
            for i in 0..size {
                buf[i] = data[cursor + i];
            }
            let v = i32::from_le_bytes(buf);
            cursor += size;
            (DexValue::Int(v), cursor)
        }
        0x06 => {
            // VALUE_LONG
            let mut buf = [0u8; 8];
            for i in 0..size {
                buf[i] = data[cursor + i];
            }
            let v = i64::from_le_bytes(buf);
            cursor += size;
            (DexValue::Long(v), cursor)
        }
        0x10 => {
            // VALUE_FLOAT
            let mut buf = [0u8; 4];
            for i in 0..size {
                buf[i] = data[cursor + i];
            }
            let v = f32::from_le_bytes(buf);
            cursor += size;
            (DexValue::Float(v), cursor)
        }
        0x11 => {
            // VALUE_DOUBLE
            let mut buf = [0u8; 8];
            for i in 0..size {
                buf[i] = data[cursor + i];
            }
            let v = f64::from_le_bytes(buf);
            cursor += size;
            (DexValue::Double(v), cursor)
        }
        0x17 => {
            // VALUE_STRING
            let mut val = 0u32;
            for i in 0..size {
                val |= (data[cursor + i] as u32) << (8 * i);
            }
            cursor += size;
            let (_, s) = container
                .string_offset(val as usize)
                .map(|off| {
                    super::strings::parse_string_at_offset(data, off, &container.header_item, 0)
                })
                .unwrap_or_else(|| (val as usize, "<unknown>".to_string()));
            (DexValue::String(s), cursor)
        }
        0x18 => {
            // VALUE_TYPE
            let mut val = 0u32;
            for i in 0..size {
                val |= (data[cursor + i] as u32) << (8 * i);
            }
            cursor += size;
            let (_, s) = container
                .type_to_string_offset(val as usize)
                .map(|off| {
                    super::strings::parse_string_at_offset(data, off, &container.header_item, 0)
                })
                .unwrap_or_else(|| (val as usize, "<unknown>".to_string()));
            (DexValue::Type(s), cursor)
        }
        0x1e => (DexValue::Null, cursor), // VALUE_NULL
        0x1f => (DexValue::Boolean(val_arg != 0), cursor), // VALUE_BOOLEAN duplicate
        _ => (DexValue::Null, cursor),    // fallback for unhandled types
    }
}

pub fn parse_code_item(data: &[u8], offset: usize) -> CodeItem {
    let mut cursor = offset;

    let registers_size = parse_u16(data, cursor);
    cursor += 2;
    let ins_size = parse_u16(data, cursor);
    cursor += 2;
    let outs_size = parse_u16(data, cursor);
    cursor += 2;
    let tries_size = parse_u16(data, cursor);
    cursor += 2;
    let debug_info_off = parse_u32(data, cursor);
    cursor += 4;
    let insns_size = parse_u32(data, cursor);
    cursor += 4;

    // Read raw instructions
    let mut insns = Vec::with_capacity((insns_size as usize) * 2);
    for _ in 0..insns_size * 2 {
        insns.push(data[cursor]);
        cursor += 1;
    }

    // Optional padding if insns_size is odd
    let padding = if insns_size % 2 != 0 {
        Some(parse_u16(data, cursor))
    } else {
        None
    };

    let instructions = parse_instructions(&insns);

    CodeItem {
        registers_size,
        ins_size,
        outs_size,
        tries_size,
        debug_info_off,
        insns_size,
        insns,
        instructions,
        padding,
    }
}

fn parse_instructions(insns: &[u8]) -> Vec<Instruction> {
    let mut instructions = Vec::new();
    let mut i: usize = 0;

    // println!("{:X?}", insns);

    while i < insns.len() {
        let opcode = insns[i];

        if i + 1 >= insns.len() {
            break
        }

        match opcode {
            0x00 => {
                // noop
                i += 1;

                instructions.push(Instruction::Nop);
            }
            0x01 => {
                // move
                i += 1;

                let destination = insns[i] >> 4;
                let source = get_lower_bits(insns[i], 4);
                i += 1;

                instructions.push(Instruction::Move {
                    dst: destination,
                    src: source,
                });
            }
            0x02 => {
                // move-from 16
                i += 1;

                let destination = insns[i];
                i += 1;

                let source = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::MoveFrom16 {
                    dst: destination,
                    src: source,
                });
            }
            0x03 => {
                // move/16
                i += 1;

                let destination = parse_u16(insns, i);
                i += 1;

                let source = parse_u16(insns, i);
                i += 1;

                instructions.push(
                    Instruction::Move16 { dst: destination, src: source }
                );
            }
            0x04 => {
                // move-wide
                i += 1;
                let destination = insns[i] >> 4;
                let source = get_lower_bits(insns[i], 4);
                i += 1;

                instructions.push(Instruction::MoveWide {
                    dst: destination,
                    src: source,
                });
            }
            0x05 => {
                // move-wide/from16
                i += 1;

                let destination = insns[i];
                i += 1;

                let source = parse_u16(insns, i);
                i += 2;

                instructions.push(
                    Instruction::MoveWideFrom16 { dst: destination, src: source }
                );
            }
            0x06 => {
                // move-wide/16
                i += 1;

                let destination = parse_u16(insns, i);
                i += 2;

                let source = parse_u16(insns, i);
                i += 2;

                instructions.push(
                    Instruction::MoveWide16 { dst: destination, src: source }
                );
            }
            0x07 => {
                // move-object
                i += 1;

                let registers_byte = insns[i];
                let source = registers_byte >> 4;
                let destination = get_lower_bits(registers_byte, 4);

                i += 1;

                instructions.push(Instruction::MoveObject {
                    dst: destination,
                    src: source,
                });
            }
            0x08 => {
                // move-object/from16
                i += 1;

                let destination = insns[i];
                i += 1;

                let source = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::MoveObjectFrom16 {
                    dst: destination,
                    src: source,
                });
            }
            0x09 => {
                // move-object/16
                i += 1;

                let destination = parse_u16(insns, i);
                i += 1;

                let source = parse_u16(insns, i);
                i += 1;

                instructions.push(
                    Instruction::MoveObject16 { dst: destination, src: source }
                );
            }
            0x10 => {
                // return wide
                i += 1;

                let ret_reg = insns[i];
                i += 1;

                instructions.push(Instruction::ReturnWide { reg: ret_reg });
            }
            0x0A => {
                // move-result
                i += 1;
                let destination = insns[i];
                i += 1;

                instructions.push(Instruction::MoveResult { dst: destination });
            }
            0x0B => {
                // move-result-wide
                i += 1;

                let destination = insns[i];
                i += 1;

                instructions.push(Instruction::MoveResultWide { dst: destination })
            }
            0x0C => {
                // move-result-object
                i += 1;
                let destination = insns[i];
                i += 1;

                instructions.push(Instruction::MoveResultObject { dst: destination });
            }
            0x0D => {
                // move-exception
                i += 1;
                let destination = insns[i];
                i += 1;

                instructions.push(Instruction::MoveException { dst: destination });
            }
            0x0E => {
                // return-void
                i += 2;

                instructions.push(Instruction::ReturnVoid);
            }
            0x0F => {
                // move-result-object
                i += 1;
                let return_value_register = insns[i];
                i += 1;

                instructions.push(Instruction::Return {
                    reg: return_value_register,
                });
            }
            0x11 => {
                // return-object
                // println!("return-object");
                i += 1;
                // next byte is register
                let return_register = insns[i];
                // println!("Return register -> {}", return_register);
                i += 1;

                instructions.push(Instruction::ReturnObject {
                    src: return_register,
                });
            }
            0x12 => {
                // const/4
                i += 1;
                let destination = get_lower_bits(insns[i], 4);
                let literal: i8 = i8::from_le_bytes([insns[i] >> 4]);

                i += 1;
                instructions.push(Instruction::Const4Bit {
                    dst: destination,
                    signed_int: literal,
                });
            }
            0x13 => {
                // const/16
                i += 1;
                let destination = insns[i];
                i += 1;

                let literal: i16 = parse_i16(insns, i);
                i += 2;

                instructions.push(Instruction::Const16Bit {
                    dst: destination,
                    signed_int: literal,
                });
            }
            0x14 => {
                // const 32 bit
                i += 1;
                let destination = insns[i];
                i += 1;

                let literal: u32 = parse_u32(insns, i);
                i += 4;

                instructions.push(Instruction::Const32Bit {
                    dst: destination,
                    literal,
                });
            }
            0x15 => {
                // const-high/16
                i += 1;
                
                let destination = insns[i];
                i += 1;

                let literal = parse_i16(insns, i);
                i += 2;

                instructions.push(
                    Instruction::ConstHigh16 { dst: destination, literal }
                );
            }
            0x16 => {
                // const-wide 16
                i += 1;
                let destination = insns[i];
                i += 1;

                let literal: i16 = parse_i16(insns, i);
                i += 2;

                instructions.push(Instruction::ConstWide16Bit {
                    dst: destination,
                    signed_int: literal,
                });
            }
            0x17 => {
                // const-wide/32
                i += 1;

                let destination = insns[i];
                i += 1;

                let literal = parse_i32(insns, i);
                i += 4;

                instructions.push(
                    Instruction::ConstWide32 { dst: destination, literal }
                );
            }
            0x18 => {
                // const-wide 64
                i += 1;
                let destination = insns[i];
                i += 1;

                let literal: u64 = parse_u64(insns, i);
                i += 8;

                instructions.push(Instruction::ConstWide64Bit {
                    dst: destination,
                    literal,
                });
            }
            0x19 => {
                // const-wide/high16
                i += 1;

                let destination = insns[i];
                i += 1;

                let literal = parse_i16(insns, i);
                i += 2;

                instructions.push(
                    Instruction::ConstWide16BitHigh { dst: destination, signed_int: literal }
                );
            }
            0x1A => {
                // const-string
                i += 1;
                let destination = insns[i];
                i += 1;

                let string_idx = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::ConstString {
                    dest: destination,
                    string_idx,
                });
            }
            0x1B => {
                // const-string/jumbo
                i += 1;

                let destination = insns[i];
                i += 1;

                let string_idx = parse_u32(insns, i);
                i += 4;

                instructions.push(
                    Instruction::ConstStringJumbo { dest: destination, string_idx }
                );
            }
            0x1C => {
                // const-class
                i += 1;

                let destination = insns[i];
                i += 1;
                let type_idx = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::ConstClass {
                    dst: destination,
                    type_idx,
                });
            }
            0x1D => {
                // monitor-enter
                i += 1;

                let ref_bearing_reg = insns[i];
                i += 1;

                instructions.push(
                    Instruction::MonitorEnter { ref_bearing_reg }
                );
            }
            0x1E => {
                // monitor-enter
                i += 1;

                let ref_bearing_reg = insns[i];
                i += 1;

                instructions.push(
                    Instruction::MonitorExit { ref_bearing_reg }
                );
            }
            0x1F => {
                // check-cast
                i += 1;
                let ref_bearing_reg = insns[i];
                i += 1;
                let type_idx = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::CheckCast {
                    ref_bearing_reg,
                    type_idx,
                });
            }
            0x20 => {
                // instance-of
                i += 1;

                let destination = insns[i] >> 4;
                let ref_bearing_reg = get_lower_bits(insns[i], 4);
                i += 1;

                let type_idx = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::InstanceOf {
                    dst: destination,
                    ref_bearing_reg,
                    type_idx,
                });
            }
            0x21 => {
                // array-length
                i += 1;

                let destination = insns[i] >> 4;
                let array_ref_bearing_reg = get_lower_bits(insns[i], 4);
                i += 1;

                instructions.push(Instruction::ArrayLength {
                    dst: destination,
                    array_ref_bearing_reg,
                });
            }
            0x22 => {
                // new-instance
                i += 1;
                let destination = insns[i];
                i += 1;
                let type_idx = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::NewInstance {
                    dst: destination,
                    type_idx,
                });
            }
            0x23 => {
                // new-array
                i += 1;

                let destination = insns[i] >> 4;
                let size = get_lower_bits(insns[i], 4);
                i += 1;

                let type_idx = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::NewArray {
                    dst: destination,
                    size,
                    type_idx,
                })
            }
            0x24 => {
                // filled-new-array
                i += 1;

                let argument_count: u8 = insns[i] >> 4;
                i += 1;

                let type_idx = parse_u16(insns, i);
                i += 2;

                let mut arg_registers: Vec<u8> = Vec::with_capacity(argument_count as usize);
                let args_slice = &insns[i..i + 2];
                i += 2;
                if argument_count > 0 {
                    for b in 0..(argument_count - 1).div_ceil(2) {
                        let argument_register: u8 = args_slice[b as usize] >> 4;
                        arg_registers.push(argument_register);
                        if argument_count % 2 == 0 {
                            let second_argument_register: u8 = get_lower_bits(args_slice[b as usize], 4);
                            arg_registers.push(second_argument_register);
                        }
                    }
                }

                instructions.push(Instruction::FilledNewArray {
                    argc: argument_count,
                    args: arg_registers,
                    type_idx,
                });
            }
            0x25 => {
                // filled-new-array/range
                i += 1;

                let count = insns[i];
                i += 1;

                let type_idx = parse_u16(insns, i);
                i += 2;

                let first_arg_reg = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::FilledNewArrayRange {
                    count,
                    type_idx,
                    first_arg_reg,
                });
            }
            0x26 => {
                // filled-array-data
                i += 1;

                let array_ref = insns[i];
                i += 1;

                let signed_fake_branch_off = parse_i32(insns, i);
                i += 1;

                instructions.push(
                    Instruction::FilledArrayData { array_ref, signed_fake_branch_off }
                );
            }
            0x27 => {
                // throw
                i += 1;

                let reg = insns[i];
                i += 1;

                instructions.push(Instruction::Throw { reg });
            }
            0x28 => {
                // goto
                i += 1;

                let signed_branch_off: i8 = i8::from_le_bytes([insns[i]]);
                i += 1;

                instructions.push(Instruction::Goto { signed_branch_off });
            }
            0x29 => {
                // goto/16
                i += 1;

                let signed_branch_off = parse_i16(insns, i);
                i += 2;

                instructions.push(Instruction::Goto16 { signed_branch_off });
            }
            0x2A => {
                i += 1;
            }
            0x2B => {
                // packed-switch
                i += 1;

                let test_reg = insns[i];
                i += 1;

                let signed_fake_branch_off = parse_i32(insns, i);
                i += 4;

                instructions.push(
                    Instruction::PackedSwitch { test_reg, signed_fake_branch_off }
                );
            }
            0x2C => {
                // sparse-switch
                i += 1;

                let test_reg = insns[i];
                i += 1;

                let signed_fake_branch_off = parse_i32(insns, i);
                i += 4;

                instructions.push(Instruction::SparseSwitch {
                    test_reg,
                    signed_fake_branch_off,
                });
            }
            0x2D..=0x31 => {
                // 2d = cmpl-float (lt bias)
                // 2e = cmpg-float (gt bias)
                // 2f = cmpl-double (lt bias)
                // 30 = cmpg-double (gt bias)
                // 31 = cmp-long
                i += 1;

                let destination = insns[i];
                i += 1;

                let first_reg = insns[i];
                i += 1;
                
                let second_reg = insns[i];
                i += 1;

                match opcode {
                    0x2D => {
                        instructions.push(
                            Instruction::CmpLessFloat { dst: destination, first_reg, second_reg }
                        );
                    }
                    0x2E => {
                        instructions.push(
                            Instruction::CmpGreaterFloat { dst: destination, first_reg, second_reg }
                        );
                    }
                    0x2F => {
                        instructions.push(
                            Instruction::CmpLessDouble { dst: destination, first_reg, second_reg }
                        );
                    }
                    0x30 => {
                        instructions.push(
                            Instruction::CmpGreaterDouble { dst: destination, first_reg, second_reg }
                        );
                    }
                    0x31 => {
                        instructions.push(
                            Instruction::CmpLong { dst: destination, first_reg, second_reg }
                        );
                    }
                    _ => {
                        assert!(false, "This should be unreachable. Opcode -> {:02X}", opcode);
                    }
                }
            }
            0x32..=0x37 => {
                // 32 = if-eq
                // 33 = if-ne
                // 34 = if-lt
                // 35 = if-ge
                // 36 = if-gt
                // 37 = if-le
                i += 1;

                let first_reg = insns[i] >> 4;
                let second_reg = get_lower_bits(insns[i], 4);
                i += 1;

                let signed_branch_off = parse_i16(insns, i);
                i += 2;

                match opcode {
                    0x32 => {
                        instructions.push(Instruction::TestIfEqual {
                            first_reg,
                            second_reg,
                            signed_branch_off,
                        });
                    }
                    0x33 => {
                        instructions.push(Instruction::TestIfNotEqual {
                            first_reg,
                            second_reg,
                            signed_branch_off,
                        });
                    }
                    0x34 => {
                        instructions.push(Instruction::TestIfLessThan {
                            first_reg,
                            second_reg,
                            signed_branch_off,
                        });
                    }
                    0x35 => {
                        instructions.push(Instruction::TestIfGreaterEqual {
                            first_reg,
                            second_reg,
                            signed_branch_off,
                        });
                    }
                    0x36 => {
                        instructions.push(Instruction::TestIfGreaterThan {
                            first_reg,
                            second_reg,
                            signed_branch_off,
                        });
                    }
                    0x37 => {
                        instructions.push(Instruction::TestIfLessEqual {
                            first_reg,
                            second_reg,
                            signed_branch_off,
                        });
                    }
                    _ => {
                        assert!(
                            false,
                            "This should be unreachable. Opcode -> 0x{:X}",
                            opcode
                        );
                    }
                }
            }
            0x38..=0x3D => {
                // 38 = if-eqz
                // 39 = if-nez
                // 3a = if-ltz
                // 3b = if-gez
                // 3c = if-gtz
                // 3d = if-lez

                i += 1;
                let test_reg = insns[i];
                i += 1;

                let branch_off = parse_i16(insns, i);
                i += 2;

                match opcode {
                    0x38 => {
                        instructions.push(Instruction::BranchIfEqualZero {
                            test_reg,
                            signed_branch_off: branch_off,
                        });
                    }
                    0x39 => {
                        instructions.push(Instruction::BranchIfNotEqualZero {
                            test_reg,
                            signed_branch_off: branch_off,
                        });
                    }
                    0x3A => {
                        instructions.push(Instruction::BranchIfLessThanZero {
                            test_reg,
                            signed_branch_off: branch_off,
                        });
                    }
                    0x3B => {
                        instructions.push(Instruction::BranchIfGreaterEqualZero {
                            test_reg,
                            signed_branch_off: branch_off,
                        });
                    }
                    0x3C => {
                        instructions.push(Instruction::BranchIfGreaterThanZero {
                            test_reg,
                            signed_branch_off: branch_off,
                        });
                    }
                    0x3D => {
                        instructions.push(Instruction::BranchIfLessEqualZero {
                            test_reg,
                            signed_branch_off: branch_off,
                        });
                    }
                    _ => {
                        assert!(false, "This should not be reachable")
                    }
                }
            }
            0x3e..=0x43 => {
                // unused
                i += 1;
            }
            0x44..=0x51 => {
                // 44 = aget
                // 45 = aget-wide
                // 46 = aget-object
                // 47 = aget-boolean
                // 48 = aget-byte
                // 49 = aget-char
                // 4a = aget-short
                // 4b = aput
                // 4c = aput-wide
                // 4d = aput-object
                // 4e = aput-boolean
                // 4f = aput-byte
                // 50 = aput-char
                // 51 = aput-short

                i += 1;
                let source = insns[i];
                i += 1;

                let array_reg = insns[i];
                i += 1;

                let index_reg = insns[i];
                i += 1;

                match opcode {
                    0x44 => {
                        instructions.push(Instruction::AGet {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x45 => {
                        instructions.push(Instruction::AGetWide {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x46 => {
                        instructions.push(Instruction::AGetObject {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x47 => {
                        instructions.push(Instruction::AGetBoolean {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x48 => {
                        instructions.push(Instruction::AGetByte {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x49 => {
                        instructions.push(Instruction::AGetChar {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x4A => {
                        instructions.push(Instruction::AGetShort {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x4B => {
                        instructions.push(Instruction::APut {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x4C => {
                        instructions.push(Instruction::APutWide {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x4D => {
                        instructions.push(Instruction::APutObject {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x4E => {
                        instructions.push(Instruction::APutBoolean {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x4F => {
                        instructions.push(Instruction::APutByte {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x50 => {
                        instructions.push(Instruction::APutChar {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    0x51 => {
                        instructions.push(Instruction::APutShort {
                            src: source,
                            array_reg,
                            index_reg,
                        });
                    }
                    _ => {
                        assert!(
                            false,
                            "This should be unreachable. opcode -> 0x{:X}",
                            opcode
                        );
                    }
                }
            }
            0x52..=0x5F => {
                // 52 = iget
                // 53 = iget-wide
                // 54 = iget-object
                // 55 = iget-boolean
                // 56 = iget-byte
                // 57 = iget-char
                // 58 = iget-short
                // 59 = iput
                // 5a = iput-wide
                // 5b = iput-object
                // 5c = iput-boolean
                // 5d = iput-byte
                // 5e = iput-char
                // 5f = iput-short
                i += 1;
                // next byte is source register and object register
                let registers_byte = insns[i];
                let source = get_lower_bits(registers_byte, 4);
                let object = registers_byte >> 4;
                // println!("Source -> {}, Object -> {}", source, object);
                i += 1;

                // next 2 bytes are instance field reference index
                let reference_index = parse_u16(insns, i);
                // println!("Instance field index -> {}", reference_index);
                i += 2;

                match opcode {
                    0x52 => {
                        instructions.push(Instruction::IGet {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x53 => {
                        instructions.push(Instruction::IGetWide {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x54 => {
                        instructions.push(Instruction::IGetObject {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x55 => {
                        instructions.push(Instruction::IGetBoolean {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x56 => {
                        instructions.push(Instruction::IGetByte {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x57 => {
                        instructions.push(Instruction::IGetChar {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x58 => {
                        instructions.push(Instruction::IGetShort {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x59 => {
                        instructions.push(Instruction::IPut {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x5A => {
                        instructions.push(Instruction::IPutWide {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x5B => {
                        instructions.push(Instruction::IPutObject {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x5C => {
                        instructions.push(Instruction::IPutBoolean {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x5D => {
                        instructions.push(Instruction::IPutByte {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x5E => {
                        instructions.push(Instruction::IPutChar {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    0x5F => {
                        instructions.push(Instruction::IPutShort {
                            src: source,
                            obj: object,
                            instance_field_idx: reference_index,
                        });
                    }
                    _ => {
                        assert!(false, "This should be unreachable. Opcode -> {:X}", opcode);
                    }
                }
            }
            0x60..=0x6D => {
                // 60 = sget
                // 61 = sget-wide
                // 62 = sget-object
                // 63 = sget-boolean
                // 64 = sget-byte
                // 65 = sget-char
                // 66 = sget-short
                // 67 = sput
                // 68 = sput-wide
                // 69 = sput-object
                // 6a = sput-boolean
                // 6b = sput-byte
                // 6c = sput-char
                // 6d = sput-short

                i += 1;
                let source = insns[i];
                i += 1;

                let static_field_ref_idx = parse_u16(insns, i);
                i += 2;

                match opcode {
                    0x60 => {
                        instructions.push(Instruction::SGet {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x61 => {
                        instructions.push(Instruction::SGetWide {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x62 => {
                        instructions.push(Instruction::SGetObject {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x63 => {
                        instructions.push(Instruction::SGetBoolean {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x64 => {
                        instructions.push(Instruction::SGetByte {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x65 => {
                        instructions.push(Instruction::SGetChar {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x66 => {
                        instructions.push(Instruction::SGetShort {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x67 => {
                        instructions.push(Instruction::SPut {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x68 => {
                        instructions.push(Instruction::SPutWide {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x69 => {
                        instructions.push(Instruction::SPutObject {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x6A => {
                        instructions.push(Instruction::SPutBoolean {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x6B => {
                        instructions.push(Instruction::SPutByte {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x6C => {
                        instructions.push(Instruction::SPutChar {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    0x6D => {
                        instructions.push(Instruction::SPutShort {
                            src: source,
                            static_field_idx: static_field_ref_idx,
                        });
                    }
                    _ => {
                        assert!(
                            false,
                            "This should be unreachable. opcode -> 0x{:X}",
                            opcode
                        );
                    }
                }
            }
            0x6E..=0x72 => {
                // 0x6e = invoke-virtual
                // 0x6f = invoke-super
                // 0x70 = invoke-direct
                // 0x71 = invoke-static
                // 0x72 = invoke-interface
                i += 1;
                let argument_count: u8 = insns[i] >> 4;
                i += 1;

                let method_ref_index = parse_u16(insns, i);
                i += 2;

                let mut arg_registers: Vec<u8> = Vec::with_capacity(argument_count as usize);
                let args_slice = &insns[i..i + 2];
                i += 2;
                if argument_count > 0 {
                    for b in 0..(argument_count - 1).div_ceil(2) {
                        let argument_register: u8 = args_slice[b as usize] >> 4;
                        arg_registers.push(argument_register);
                        if argument_count % 2 == 0 {
                            let second_argument_register: u8 = get_lower_bits(args_slice[b as usize], 4);
                            arg_registers.push(second_argument_register);
                        }
                    }
                }

                match opcode {
                    0x6e => {
                        instructions.push(Instruction::InvokeVirtual {
                            argc: argument_count,
                            args: arg_registers,
                            method_idx: method_ref_index,
                        });
                    }
                    0x6f => {
                        instructions.push(Instruction::InvokeSuper {
                            argc: argument_count,
                            args: arg_registers,
                            method_idx: method_ref_index,
                        });
                    }
                    0x70 => {
                        instructions.push(Instruction::InvokeDirect {
                            argc: argument_count,
                            args: arg_registers,
                            method_idx: method_ref_index,
                        });
                    }
                    0x71 => {
                        instructions.push(Instruction::InvokeStatic {
                            argc: argument_count,
                            args: arg_registers,
                            method_idx: method_ref_index,
                        });
                    }
                    0x72 => {
                        instructions.push(Instruction::InvokeInterface {
                            argc: argument_count,
                            args: arg_registers,
                            method_idx: method_ref_index,
                        });
                    }
                    _ => {
                        assert!(false, "This should be unreachable. Opcode -> {:X}", opcode);
                    }
                }
            }
            0x73 => {
                // unused
                i += 1;
            }
            0x74..=0x78 => {
                // 74 = invoke-virtual/range
                // 75 = invoke-super/range
                // 76 = invoke-direct/range
                // 77 = invoke-static/range
                // 78 = invoke-interface/range
                i += 1;

                let count = insns[i];
                i += 1;

                let type_idx = parse_u16(insns, i);
                i += 2;

                let first_arg_reg = parse_u16(insns, i);
                i += 2;

                match opcode {
                    0x74 => {
                        instructions.push(Instruction::InvokeVirtualRange {
                            count,
                            type_idx,
                            first_arg_reg,
                        });
                    }
                    0x75 => {
                        instructions.push(Instruction::InvokeSuperRange {
                            count,
                            type_idx,
                            first_arg_reg,
                        });
                    }
                    0x76 => {
                        instructions.push(Instruction::InvokeDirectRange {
                            count,
                            type_idx,
                            first_arg_reg,
                        });
                    }
                    0x77 => {
                        instructions.push(Instruction::InvokeStaticRange {
                            count,
                            type_idx,
                            first_arg_reg,
                        });
                    }
                    0x78 => {
                        instructions.push(Instruction::InvokeInterfaceRange {
                            count,
                            type_idx,
                            first_arg_reg,
                        });
                    }
                    _ => {
                        assert!(false, "This should be unreachable. Opcode -> {:X}", opcode);
                    }
                }
            }
            0x79..=0x7A => {
                // unused
                i += 1;
            }
            0x7B..=0x8F => {
                // 7b = neg-int
                // 7c = not-int
                // 7d = neg-long
                // 7e = not-long
                // 7f = neg-float
                // 80 = neg-double
                // 81 = int-to-long
                // 82 = int-to-float
                // 83 = int-to-double
                // 84 = long-to-int
                // 85 = long-to-float
                // 86 = long-to-double
                // 87 = float-to-int
                // 88 = float-to-long
                // 89 = float-to-double
                // 8a = double-to-int
                // 8b = double-to-long
                // 8c = double-to-float
                // 8d = int-to-byte
                // 8e = int-to-char
                // 8f = int-to-short
                i += 1;

                let destination = insns[i] >> 4;
                let source = get_lower_bits(insns[i], 4);

                i += 1;

                match opcode {
                    0x7b => {
                        instructions.push(Instruction::NegInt {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x7c => {
                        instructions.push(Instruction::NotInt {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x7d => {
                        instructions.push(Instruction::NegLong {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x7e => {
                        instructions.push(Instruction::NotLong {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x7f => {
                        instructions.push(Instruction::NegFloat {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x80 => {
                        instructions.push(Instruction::NegDouble {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x81 => {
                        instructions.push(Instruction::IntToLong {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x82 => {
                        instructions.push(Instruction::IntToFloat {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x83 => {
                        instructions.push(Instruction::IntToDouble {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x84 => {
                        instructions.push(Instruction::LongToInt {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x85 => {
                        instructions.push(Instruction::LongToFloat {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x86 => {
                        instructions.push(Instruction::LongToDouble {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x87 => {
                        instructions.push(Instruction::FloatToInt {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x88 => {
                        instructions.push(Instruction::FloatToLong {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x89 => {
                        instructions.push(Instruction::FloatToDouble {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x8a => {
                        instructions.push(Instruction::DoubleToInt {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x8b => {
                        instructions.push(Instruction::DoubleToLong {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x8c => {
                        instructions.push(Instruction::DoubleToFloat {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x8d => {
                        instructions.push(Instruction::IntToByte {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x8e => {
                        instructions.push(Instruction::IntToChar {
                            dst: destination,
                            src: source,
                        });
                    }
                    0x8f => {
                        instructions.push(Instruction::IntToShort {
                            dst: destination,
                            src: source,
                        });
                    }
                    _ => {
                        assert!(
                            false,
                            "This code should not be reachable. Opcode -> 0x{:X}",
                            opcode
                        );
                    }
                }
            }
            0x90..=0xAF => {
                // 90 = add-int
                // 91 = sub-int
                // 92 = mul-int
                // 93 = div-int
                // 94 = rem-int
                // 95 = and-int
                // 96 = or-int
                // 97 = xor-int
                // 98 = shl-int
                // 99 = shr-int
                // 9a = ushr-int
                // 9b = add-long
                // 9c = sub-long
                // 9d = mul-long
                // 9e = div-long
                // 9f = rem-long
                // a0 = and-long
                // a1 = or-long
                // a2 = xor-long
                // a3 = shl-long
                // a4 = shr-long
                // a5 = ushr-long
                // a6 = add-float
                // a7 = sub-float
                // a8 = mul-float
                // a9 = div-float
                // aa = rem-float
                // ab = add-double
                // ac = sub-double
                // ad = mul-double
                // ae = div-double
                // af = rem-double
                i += 1;

                let destination = insns[i];
                i += 1;

                let first_src = insns[i];
                i += 1;

                let second_src = insns[i];
                i += 1;

                match opcode {
                    0x90 => {
                        instructions.push(Instruction::AddInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x91 => {
                        instructions.push(Instruction::SubInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x92 => {
                        instructions.push(Instruction::MulInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x93 => {
                        instructions.push(Instruction::DivInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x94 => {
                        instructions.push(Instruction::RemInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x95 => {
                        instructions.push(Instruction::AndInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x96 => {
                        instructions.push(Instruction::OrInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x97 => {
                        instructions.push(Instruction::XorInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x98 => {
                        instructions.push(Instruction::ShLInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x99 => {
                        instructions.push(Instruction::ShRInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x9a => {
                        instructions.push(Instruction::UShRInt {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x9b => {
                        instructions.push(Instruction::AddLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x9c => {
                        instructions.push(Instruction::SubLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x9d => {
                        instructions.push(Instruction::MulLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x9e => {
                        instructions.push(Instruction::DivLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0x9f => {
                        instructions.push(Instruction::RemLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xa0 => {
                        instructions.push(Instruction::AndLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xa1 => {
                        instructions.push(Instruction::OrLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xa2 => {
                        instructions.push(Instruction::XorLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xa3 => {
                        instructions.push(Instruction::ShLLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xa4 => {
                        instructions.push(Instruction::ShRLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xa5 => {
                        instructions.push(Instruction::UShRLong {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xa6 => {
                        instructions.push(Instruction::AddFloat {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xa7 => {
                        instructions.push(Instruction::SubFloat {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xa8 => {
                        instructions.push(Instruction::MulFloat {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xa9 => {
                        instructions.push(Instruction::DivFloat {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xaa => {
                        instructions.push(Instruction::RemFloat {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xab => {
                        instructions.push(Instruction::AddDouble {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xac => {
                        instructions.push(Instruction::SubDouble {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xad => {
                        instructions.push(Instruction::MulDouble {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xae => {
                        instructions.push(Instruction::DivDouble {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    0xaf => {
                        instructions.push(Instruction::RemDouble {
                            dst: destination,
                            first_src,
                            second_src,
                        });
                    }
                    _ => {
                        assert!(false, "This should be unreachable. Opcode -> {:X}", opcode);
                    }
                }
            }
            0xB0..=0xCF => {
                // b0 = add-int/2addr
                // b1 = sub-int/2addr
                // b2 = mul-int/2addr
                // b3 = div-int/2addr
                // b4 = rem-int/2addr
                // b5 = and-int/2addr
                // b6 = or-int/2addr
                // b7 = xor-int/2addr
                // b8 = shl-int/2addr
                // b9 = shr-int/2addr
                // ba = ushr-int/2addr
                // bb = add-long/2addr
                // bc = sub-long/2addr
                // bd = mul-long/2addr
                // be = div-long/2addr
                // bf = rem-long/2addr
                // c0 = and-long/2addr
                // c1 = or-long/2addr
                // c2 = xor-long/2addr
                // c3 = shl-long/2addr
                // c4 = shr-long/2addr
                // c5 = ushr-long/2addr
                // c6 = add-float/2addr
                // c7 = sub-float/2addr
                // c8 = mul-float/2addr
                // c9 = div-float/2addr
                // ca = rem-float/2addr
                // cb = add-double/2addr
                // cc = sub-double/2addr
                // cd = mul-double/2addr
                // ce = div-double/2addr
                // cf = rem-double/2addr
                i += 1;

                let dst_and_first_src = insns[i] >> 4;
                let second_src = get_lower_bits(insns[i], 4);
                i += 1;

                match opcode {
                    0xB0 => instructions.push(Instruction::AddInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xB1 => instructions.push(Instruction::SubInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xB2 => instructions.push(Instruction::MulInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xB3 => instructions.push(Instruction::DivInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xB4 => instructions.push(Instruction::RemInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xB5 => instructions.push(Instruction::AndInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xB6 => instructions.push(Instruction::OrInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xB7 => instructions.push(Instruction::XorInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xB8 => instructions.push(Instruction::ShlInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xB9 => instructions.push(Instruction::ShrInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xBA => instructions.push(Instruction::UshrInt2Addr {
                        dst_and_first_src,
                        second_src,
                    }),

                    0xBB => instructions.push(Instruction::AddLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xBC => instructions.push(Instruction::SubLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xBD => instructions.push(Instruction::MulLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xBE => instructions.push(Instruction::DivLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xBF => instructions.push(Instruction::RemLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xC0 => instructions.push(Instruction::AndLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xC1 => instructions.push(Instruction::OrLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xC2 => instructions.push(Instruction::XorLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xC3 => instructions.push(Instruction::ShlLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xC4 => instructions.push(Instruction::ShrLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xC5 => instructions.push(Instruction::UshrLong2Addr {
                        dst_and_first_src,
                        second_src,
                    }),

                    0xC6 => instructions.push(Instruction::AddFloat2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xC7 => instructions.push(Instruction::SubFloat2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xC8 => instructions.push(Instruction::MulFloat2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xC9 => instructions.push(Instruction::DivFloat2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xCA => instructions.push(Instruction::RemFloat2Addr {
                        dst_and_first_src,
                        second_src,
                    }),

                    0xCB => instructions.push(Instruction::AddDouble2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xCC => instructions.push(Instruction::SubDouble2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xCD => instructions.push(Instruction::MulDouble2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xCE => instructions.push(Instruction::DivDouble2Addr {
                        dst_and_first_src,
                        second_src,
                    }),
                    0xCF => instructions.push(Instruction::RemDouble2Addr {
                        dst_and_first_src,
                        second_src,
                    }),

                    _ => {
                        panic!("This should be unreachable. Opcode -> 0x{:X}", opcode);
                    }
                }
            }
            0xD0..=0xD7 => {
                // d0 = add-int/lit16
                // d1 = rsub-int (reverse subtract)
                // d2 = mul-int/lit16
                // d3 = div-int/lit16
                // d4 = rem-int/lit16
                // d5 = and-int/lit16
                // d6 = or-int/lit16
                // d7 = xor-int/lit16
                i += 1;

                let destination = insns[i] >> 4;
                let source = get_lower_bits(insns[i], 4);
                i += 1;

                let literal = parse_i16(insns, i);
                i += 2;

                match opcode {
                    0xD0 => {
                        instructions.push(
                            Instruction::AddIntLit16 { dst: destination, src: source, literal }
                        );
                    }
                    0xD1 => {
                        instructions.push(
                            Instruction::RSubIntLit16 { dst: destination, src: source, literal }
                        );
                    }
                    0xD2 => {
                        instructions.push(
                            Instruction::MulIntLit16 { dst: destination, src: source, literal }
                        );
                    }
                    0xD3 => {
                        instructions.push(
                            Instruction::DivIntLit16 { dst: destination, src: source, literal }
                        );
                    }
                    0xD4 => {
                        instructions.push(
                            Instruction::RemIntLit16 { dst: destination, src: source, literal }
                        );
                    }
                    0xD5 => {
                        instructions.push(
                            Instruction::AndIntLit16 { dst: destination, src: source, literal }
                        );
                    }
                    0xD6 => {
                        instructions.push(
                            Instruction::OrIntLit16 { dst: destination, src: source, literal }
                        );
                    }
                    0xD7 => {
                        instructions.push(
                            Instruction::XorIntLit16 { dst: destination, src: source, literal }
                        );
                    }
                    _ => {
                        assert!(false, "This should be unreachable. Opcode -> {:02X}", opcode);
                    }
                }
            }
            0xD8..=0xE2 => {
                // d8 = add-int/lit8
                // d9 = rsub-int/lit8
                // da = mul-int/lit8
                // db = div-int/lit8
                // dc = rem-int/lit8
                // dd = and-int/lit8
                // de = or-int/lit8
                // df = xor-int/lit8
                // e0 = shl-int/lit8
                // e1 = shr-int/lit8
                // e2 = ushr-int/lit8

                i += 1;
                let destination = insns[i];
                i += 1;

                let source = insns[i];
                i += 1;

                let signed_int_const: i8 = i8::from_le_bytes([insns[i]]);
                i += 1;

                match opcode {
                    0xD8 => {
                        instructions.push(Instruction::AddInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    0xD9 => {
                        instructions.push(Instruction::RSubInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    0xDA => {
                        instructions.push(Instruction::MulInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    0xDB => {
                        instructions.push(Instruction::DivInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    0xDC => {
                        instructions.push(Instruction::RemInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    0xDD => {
                        instructions.push(Instruction::AndInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    0xDE => {
                        instructions.push(Instruction::OrInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    0xDF => {
                        instructions.push(Instruction::XorInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    0xE0 => {
                        instructions.push(Instruction::ShLInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    0xE1 => {
                        instructions.push(Instruction::ShRInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    0xE2 => {
                        instructions.push(Instruction::UShRInt8Lit8 {
                            dst: destination,
                            src: source,
                            signed_int_const,
                        });
                    }
                    _ => {
                        assert!(false, "This should be unreachable.")
                    }
                }
            }
            0xE3..=0xF9 => {
                // unused
                i += 1;
            }
            0xFA => {
                // TODO: invoke-polymorphic
                i += 1;
            }
            0xFB => {
                // TODO: invoke-polymorphic/range
                i += 1;
            }
            0xFC => {
                // TODO: invoke-custom
                i += 1;
            }
            0xFD => {
                // invoke-custom/range
                i += 1;

                let count = insns[i];
                i += 1;

                let call_site_ref = parse_u16(insns, i);
                i += 2;

                let first_arg_reg = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::InvokeCustomRange {
                    count,
                    call_site_ref,
                    first_arg_reg,
                });
            }
            0xFE => {
                // const-method-handle
                i += 1;

                let destination = insns[i];
                i += 1;

                let method_handle_idx = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::ConstMethodHandle {
                    dst: destination,
                    method_handle_idx,
                });
            }
            0xFF => {
                // const-method-type
                i += 1;

                let destination = insns[i];
                i += 1;

                let method_proto_ref = parse_u16(insns, i);
                i += 2;

                instructions.push(Instruction::ConstMethodType {
                    dst: destination,
                    method_proto_ref,
                });
            }
            _ => {
                assert!(
                    false,
                    "Unknown opcode: 0x{:02b} (0x{:X}) at i: {}",
                    opcode, opcode, i
                );
            }
        }

        // i += 1
    }

    instructions
}

fn parse_parameters(data: &[u8], parameter_off: u32, container: &DexContainer) -> Vec<String> {
    let mut parameters: Vec<String> = Vec::new();

    let mut offset = (parameter_off as usize)
        .checked_sub(container.header_item.data_off as usize)
        .expect("String offset is before data section");

    // size: u32
    let size = parse_u32(data, offset);
    offset += 4;

    // list: type_item[] -> type_item: u16
    for _ in 0..size {
        let type_idx = parse_u16(data, offset);
        offset += 2;
        // parse string at type_id
        let string_off = container.type_to_string_offset(type_idx as usize).expect("Type_item string offset failed.");
        parameters.push(parse_string_at_offset(data, string_off, &container.header_item, type_idx as usize).1);
    }

    parameters
}

pub fn parse_class_data(
    data: &[u8],
    class_def: &Class_Def_Item,
    container: &DexContainer,
) -> DexClass {
    if class_def.class_data_off == 0 {
        return DexClass {
            name: "<unknown>".to_string(),
            super_class: None,
            static_fields: HashMap::new(),
            instance_fields: HashMap::new(),
            methods: HashMap::new(),
        };
    }

    let mut cursor = (class_def.class_data_off as usize)
        .checked_sub(container.header_item.data_off as usize)
        .expect("String offset is before data section");

    // 1️⃣ Read field and method counts
    let (static_fields_size, c) = read_uleb128(data, cursor);
    cursor = c;
    let (instance_fields_size, c) = read_uleb128(data, cursor);
    cursor = c;
    let (direct_methods_size, c) = read_uleb128(data, cursor);
    cursor = c;
    let (virtual_methods_size, c) = read_uleb128(data, cursor);
    cursor = c;

    // 2️⃣ Resolve class name and superclass
    let (_, class_name) = container
        .type_to_string_offset(class_def.class_idx as usize)
        .map(|off| super::strings::parse_string_at_offset(data, off, &container.header_item, 0))
        .unwrap_or_else(|| (0, "<unknown>".to_string()));

    let mut super_class_name: Option<String> = None;
    if class_def.superclass_idx != NO_INDEX {
        super_class_name = Some(container
            .type_to_string_offset(class_def.superclass_idx as usize)
            .map(|off| super::strings::parse_string_at_offset(data, off, &container.header_item, 0)).unwrap().1);
    }

    // 3️⃣ Parse static fields
    let mut static_fields: HashMap<String, DexField> = HashMap::new();
    let mut prev_field_idx = 0;
    for _ in 0..static_fields_size {
        let (field_idx_diff, c) = read_uleb128(data, cursor);
        cursor = c;
        let (access_flags, c) = read_uleb128(data, cursor);
        cursor = c;
        let field_idx = prev_field_idx + field_idx_diff;
        prev_field_idx = field_idx;

        if let Some(field_id) = container.field_id_items.get(field_idx as usize) {
            let (_, field_name) = container
                .string_offset(field_id.name_idx as usize)
                .map(|off| {
                    super::strings::parse_string_at_offset(data, off, &container.header_item, 0)
                })
                .unwrap_or_else(|| (0, "<unknown>".to_string()));
            let (_, field_type) = container
                .type_to_string_offset(field_id.type_idx as usize)
                .map(|off| {
                    super::strings::parse_string_at_offset(data, off, &container.header_item, 0)
                })
                .unwrap_or_else(|| (0, "<unknown>".to_string()));

            let field = DexField {
                ty: field_type,
                value: None, // TODO: parse from class_def.static_values_off
                is_static: true,
            };

            if class_def.static_values_off != 0 {
                let (static_values, _) =
                    parse_encoded_array(data, class_def.static_values_off as usize, container);

                for (i, field) in static_fields.values_mut().enumerate() {
                    if let Some(val) = static_values.get(i) {
                        field.value = Some(val.clone());
                    }
                }
            }
            static_fields.insert(field_name.clone(), field);
        }
    }

    // 4️⃣ Parse instance fields
    prev_field_idx = 0;
    let mut instance_fields: HashMap<String, DexField> = HashMap::new();
    for i in 0..instance_fields_size {
        let (field_idx_diff, c) = read_uleb128(data, cursor);
        cursor = c;
        let (access_flags, c) = read_uleb128(data, cursor);
        cursor = c;
        let field_idx = prev_field_idx + field_idx_diff;
        prev_field_idx = field_idx;

        if let Some(field_id) = container.field_id_items.get(field_idx as usize) {
            let (_, field_name) = container
                .string_offset(field_id.name_idx as usize)
                .map(|off| {
                    super::strings::parse_string_at_offset(data, off, &container.header_item, 0)
                })
                .unwrap_or_else(|| (0, "<unknown>".to_string()));
            let (_, field_type) = container
                .type_to_string_offset(field_id.type_idx as usize)
                .map(|off| {
                    super::strings::parse_string_at_offset(data, off, &container.header_item, 0)
                })
                .unwrap_or_else(|| (0, "<unknown>".to_string()));

            instance_fields.insert(
                field_name.clone(),
                DexField {
                    ty: field_type,
                    value: None,
                    is_static: false,
                },
            );
        }
    }

    // 5️⃣ Parse methods
    let mut methods = HashMap::new();
    let mut parse_methods = |count: u32, cursor: &mut usize, prev_method_idx: &mut u32| {
        for _ in 0..count {
            let (method_idx_diff, c) = read_uleb128(data, *cursor);
            *cursor = c;
            let (access_flags, c) = read_uleb128(data, *cursor);
            *cursor = c;
            let (code_off, c) = read_uleb128(data, *cursor);
            *cursor = c;
            let method_idx = *prev_method_idx + method_idx_diff;
            *prev_method_idx = method_idx;

            if let Some(method_id) = container.method_id_items.get(method_idx as usize) {
                let (_, method_name) = container
                    .string_offset(method_id.name_idx as usize)
                    .map(|off| {
                        super::strings::parse_string_at_offset(data, off, &container.header_item, 0)
                    })
                    .unwrap_or_else(|| (0, "<unknown>".to_string()));
                let proto = &container.proto_id_items[method_id.proto_idx as usize];
                let (_, return_type) = container
                    .type_to_string_offset(proto.return_type_idx as usize)
                    .map(|off| {
                        super::strings::parse_string_at_offset(data, off, &container.header_item, 0)
                    })
                    .unwrap_or_else(|| (0, "<unknown>".to_string()));

                let mut parameters: Vec<String> = Vec::new();
                if proto.parameters_off != 0 {
                    parameters = parse_parameters(data, proto.parameters_off, container);
                }

                let mut instructions: Vec<Instruction> = Vec::new();
                let mut registers: u16 = 0;
                if code_off != 0 {
                    let code_item_off = (code_off as usize)
                        .checked_sub(container.header_item.data_off as usize)
                        .expect("String offset is before data section");
                    let code_item = parse_code_item(data, code_item_off);
                    instructions = code_item.instructions;
                    registers = code_item.registers_size;
                }

                methods.insert(
                    method_name.clone(),
                    DexMethod {
                        name: method_name,
                        return_type,
                        parameters,
                        registers,
                        instructions: instructions, // TODO: parse actual bytecode from code_off
                    },
                );
            }
        }
    };

    let mut prev_method_idx = 0;
    parse_methods(direct_methods_size, &mut cursor, &mut prev_method_idx);
    prev_method_idx = 0;
    parse_methods(virtual_methods_size, &mut cursor, &mut prev_method_idx);

    DexClass {
        name: class_name,
        super_class: super_class_name,
        static_fields,
        instance_fields,
        methods,
    }
}

pub fn get_name_of_class(
    id: usize,
    data: &[u8],
    header_item: &Header_Item,
    type_id_items: &Vec<u32>,
    string_id_items: &Vec<u32>,
) -> String {
    let string_idx = type_id_items[id] as usize;
    let string_offset = string_id_items[string_idx];
    super::strings::parse_string_at_offset(data, string_offset, header_item, string_idx).1
}

/// Parse a simple const-string getter method into DexValue::String
pub fn parse_const_string_method(
    container: &DexContainer,
    data: &[u8],
    instructions: &[u16],
) -> Option<DexValue> {
    let mut cursor = 0;
    let mut registers: HashMap<u8, DexValue> = HashMap::new();

    while cursor < instructions.len() {
        let word = instructions[cursor];
        let opcode = (word & 0xFF) as u8;
        let arg = (word >> 8) as u8;

        match opcode {
            0x1A | 0x1B | 0x1C | 0x54 => {
                // const-string (21c)
                cursor += 1;
                if cursor >= instructions.len() {
                    return None;
                }
                let string_idx = instructions[cursor] as usize;
                let (i, s) = container
                    .string_offset(string_idx)
                    .map(|off| {
                        super::strings::parse_string_at_offset(data, off, &container.header_item, string_idx)
                    })
                    .unwrap_or_else(|| (string_idx, format!("<string:{}>", string_idx)));
                registers.insert(arg, DexValue::String(s));
            }
            0x0e | 0x11 => {
                // return-object
                return registers.get(&arg).cloned();
            }
            _ => return None, // unsupported
        }

        cursor += 1;
    }

    None
}
