use crate::parser::uleb::read_uleb128;
use crate::types::Header_Item;
use crate::utils::{save_strings_to_file, convert_vec_u8_to_vec_u16};

pub fn parse_string_at_offset(data: &[u8], string_offset: u32, header_item: &Header_Item, i: usize) -> (usize, String) {
    let offset = (string_offset as usize).checked_sub(header_item.data_off as usize).expect("String offset before data section");
    let (_utf16_size, mut cursor) = read_uleb128(data, offset);

    let mut string_bytes: Vec<u8> = Vec::new();
    while cursor < data.len() && data[cursor] != 0 {
        let mut byte = data[cursor];
        if byte == 0xC0 && data.get(cursor + 1) == Some(&0x80) {
            byte = 0;
            cursor += 1;
        }
        string_bytes.push(byte);
        cursor += 1;
    }

    (i, String::from_utf8(string_bytes).unwrap_or_else(|_| "UTF-8 decode failed".to_string()))
}

pub fn parse_strings(data: &[u8], string_id_items: &Vec<u32>, header_item: &Header_Item) {
    let all_strings: Vec<String> = string_id_items.iter()
        .enumerate()
        .map(|(i, off)| parse_string_at_offset(data, *off, header_item, i).1)
        .collect();

    save_strings_to_file(all_strings, "dex_strings.txt").expect("Failed to write strings");
}
