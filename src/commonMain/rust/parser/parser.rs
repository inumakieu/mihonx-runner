use crate::types::{Class_Def_Item, DexClass, DexContainer, Field_Id_Item, Header_Item, Method_Id_Item, Proto_Id_Item};
use crate::utils::{convert_vec_u8_to_vec_u32, save_container_to_file, save_data_to_file, save_strings_to_file};
use crate::parser::uleb::read_uleb128;
use crate::parser::class::get_name_of_class;
use crate::parser::class::parse_class_data;
use crate::utils::save_class_to_file;
use crate::utils::load_classes_from_file;
use crate::utils::load_container_from_file;
use crate::utils::load_data_from_file;
use crate::utils::load_strings_from_file;

pub struct Parser {
    pub bytes: Vec<u8>,
    pub debug_enabled: bool,
    pub data: Vec<u8>,
    pub container: Option<DexContainer>,
    pub strings: Vec<String>,
    pub classes: Vec<DexClass>,
    pub cursor: usize,
}

#[macro_export]
macro_rules! parser_log {
    ($parser:expr, $($arg:tt)*) => {
        if $parser.debug_enabled {
            println!($($arg)*);
        }
    };
}

impl Parser {
    pub fn new(bytes: Vec<u8>, debug_enabled: bool) -> Self {
        Self {
            bytes,
            debug_enabled,
            data: Vec::new(),
            container: None,
            strings: Vec::new(),
            classes: Vec::new(),
            cursor: 0
        }
    }

    pub fn initialize_from_files() -> Self {
        let data = load_data_from_file("extension.data").expect("Extension.data not found.");
        let container = Some(load_container_from_file("extension_container.json").expect("Extension_container.json not found."));
        let strings = load_strings_from_file("extension.txt").expect("Extension.txt not found.");
        let classes = load_classes_from_file().expect("ELoading classes failed.");
        Self {
            bytes: Vec::new(),
            debug_enabled: true,
            data,
            container,
            strings,
            classes,
            cursor: 0
        }
    }

    pub fn parse(&mut self) {
        parser_log!(self, "Parsing Header item.");
        let header_item: Header_Item = self.parse_header();

        parser_log!(self, "Parsing string_id_items.");
        let string_id_items = self.parse_ids_array(header_item.string_ids_size as usize * 4);

        parser_log!(self, "Parsing type_id_items.");
        let type_id_items = self.parse_ids_array(header_item.type_ids_size as usize * 4);

        parser_log!(self, "Parsing proto_id_items.");
        let proto_id_items = self.parse_proto_id_array(header_item.proto_ids_size);

        parser_log!(self, "Parsing field_id_items.");
        let field_id_items = self.parse_field_id_array(header_item.field_ids_size);

        parser_log!(self, "Parsing method_id_items.");
        let method_id_items = self.parse_method_id_array(header_item.method_ids_size);

        parser_log!(self, "Parsing class_defs.");
        let class_defs = self.parse_class_defs_array(header_item.class_defs_size);

        self.container = Some(
            DexContainer {
                header_item: header_item.clone(),
                string_id_items: string_id_items.clone(),
                type_id_items: type_id_items.clone(),
                proto_id_items: proto_id_items.clone(),
                field_id_items: field_id_items.clone(),
                method_id_items: method_id_items.clone(),
                class_defs_items: class_defs.clone(),
            }
        );

        parser_log!(self, "Parsing Data section.");
        self.data = self.bytes[(header_item.data_off as usize)..(header_item.data_off + header_item.data_size) as usize].try_into().unwrap();

        self.strings = string_id_items.iter()
            .enumerate()
            .map(|(i, off)| {
                let string = self.parse_string_at_offset(*off, &header_item);
                parser_log!(self, "Index of string: {}, string -> {}", i, string);
                string
            })
            .collect();

        self.parse_class_items();

        // store other parser data to create a parser on demand from on disk data
        let _ = save_container_to_file(&self.container.clone().unwrap(), "extension_container.json");
        let _ = save_data_to_file(self.data.clone(), "extension.data");
        let _ = save_strings_to_file(self.strings.clone(), "extension.txt");
    }

    pub fn parse_ids_array(&mut self, size: usize) -> Vec<u32> {
        let mut id_bytes = self.get_bytes_vec(size);
        convert_vec_u8_to_vec_u32(&mut id_bytes).expect("Vec<u8> to Vec<u32> conversion failed.")
    }

    pub fn parse_proto_id_array(&mut self, length: u32) -> Vec<Proto_Id_Item> {
        let mut return_vec: Vec<Proto_Id_Item> = Vec::new();
        for _ in 0..(length as usize) {
            let proto_id_item = Proto_Id_Item {
                shorty_idx: u32::from_le_bytes(self.get_bytes::<4>()),
                return_type_idx: u32::from_le_bytes(self.get_bytes::<4>()),
                parameters_off: u32::from_le_bytes(self.get_bytes::<4>()),
            };
            return_vec.push(proto_id_item);
        }
        return return_vec;
    }

    pub fn parse_field_id_array(&mut self, length: u32) -> Vec<Field_Id_Item> {
        let mut return_vec: Vec<Field_Id_Item> = Vec::new();
        for _ in 0..(length as usize) {
            let field_id_item = Field_Id_Item {
                class_idx: u16::from_le_bytes(self.get_bytes::<2>()),
                type_idx: u16::from_le_bytes(self.get_bytes::<2>()),
                name_idx: u32::from_le_bytes(self.get_bytes::<4>()),
            };
            return_vec.push(field_id_item);
        }
        return return_vec;
    }

    pub fn parse_method_id_array(&mut self, length: u32) -> Vec<Method_Id_Item> {
        let mut return_vec: Vec<Method_Id_Item> = Vec::new();
        for _ in 0..(length as usize) {
            let method_id_item = Method_Id_Item {
                class_idx: u16::from_le_bytes(self.get_bytes::<2>()),
                proto_idx: u16::from_le_bytes(self.get_bytes::<2>()),
                name_idx: u32::from_le_bytes(self.get_bytes::<4>()),
            };
            return_vec.push(method_id_item);
        }
        return return_vec;
    }

    pub fn parse_class_defs_array(&mut self, length: u32) -> Vec<Class_Def_Item> {
        let mut return_vec: Vec<Class_Def_Item> = Vec::new();
        for _ in 0..(length as usize) {
            let class_def_item = Class_Def_Item {
                class_idx: u32::from_le_bytes(self.get_bytes::<4>()),
                access_flags: u32::from_le_bytes(self.get_bytes::<4>()),
                superclass_idx: u32::from_le_bytes(self.get_bytes::<4>()),
                interfaces_off: u32::from_le_bytes(self.get_bytes::<4>()),
                source_file_idx: u32::from_le_bytes(self.get_bytes::<4>()),
                annotations_off: u32::from_le_bytes(self.get_bytes::<4>()),
                class_data_off: u32::from_le_bytes(self.get_bytes::<4>()),
                static_values_off: u32::from_le_bytes(self.get_bytes::<4>()),
            };
            return_vec.push(class_def_item);
        }
        return return_vec;
    }

    pub fn parse_string_at_offset(&self, string_offset: u32, header_item: &Header_Item) -> String {
        let offset = (string_offset as usize).checked_sub(header_item.data_off as usize).expect("String offset before data section");
        let (_utf16_size, mut cursor) = read_uleb128(&self.data, offset);

        let mut string_bytes: Vec<u8> = Vec::new();
        while cursor < self.data.len() && self.data[cursor] != 0 {
            let mut byte = self.data[cursor];
            if byte == 0xC0 && self.data.get(cursor + 1) == Some(&0x80) {
                byte = 0;
                cursor += 1;
            }
            string_bytes.push(byte);
            cursor += 1;
        }

        String::from_utf8(string_bytes).unwrap_or_else(|_| "UTF-8 decode failed".to_string())
    }

    pub fn parse_strings(&self, string_id_items: &Vec<u32>, header_item: &Header_Item) -> Vec<String> {
        let all_strings: Vec<String> = string_id_items.iter()
            .map(|off| self.parse_string_at_offset(*off, header_item))
            .collect();

        // save_strings_to_file(&all_strings, "dex_strings.txt").expect("Failed to write strings");
        return all_strings
    }

    pub fn parse_class_items(&mut self) {
        match &self.container {
            Some(container) => {
                for class_def in &container.class_defs_items {
                    // Safety check
                    if class_def.class_data_off == 0 {
                        continue;
                    }

                    let name = get_name_of_class(
                        class_def.class_idx as usize,
                        &self.data,
                        &container.header_item,
                        &container.type_id_items,
                        &container.string_id_items,
                    );
                    let super_class = get_name_of_class(
                        class_def.superclass_idx as usize,
                        &self.data,
                        &container.header_item,
                        &container.type_id_items,
                        &container.string_id_items,
                    );

                    // TODO: Support filter
                    parser_log!(self, "Parsing class -> {}", name);
                    let dex_class = parse_class_data(&self.data, class_def, &container);
                    self.classes.push(dex_class.clone());

                    parser_log!(self, "Saving class -> {}", name);
                    save_class_to_file(&dex_class, &dex_class.name).expect("Saving class to file failed.");
                }
            
            },
            None => parser_log!(self, "DexContainer is empty."),
        }
    }

    // CONST is screwing me over, so im using this workaround now 
    pub fn get_bytes_vec(&mut self, size: usize) -> Vec<u8> {
        let old_cursor = self.cursor;
        self.cursor += size;
        self.bytes[old_cursor..self.cursor].try_into().unwrap()
    }

    pub fn get_bytes<const count: usize>(&mut self) -> [u8; count] {
        let old_cursor = self.cursor;
        self.cursor += count;
        self.bytes[old_cursor..self.cursor].try_into().unwrap()
    }

    pub fn parse_header(&mut self) -> Header_Item {
        Header_Item {
            magic: self.get_bytes::<8>(),
            checksum: u32::from_le_bytes(self.get_bytes::<4>()),
            signature: self.get_bytes::<20>(),
            file_size: u32::from_le_bytes(self.get_bytes::<4>()),
            header_size: u32::from_le_bytes(self.get_bytes::<4>()),
            endian_tag: u32::from_le_bytes(self.get_bytes::<4>()),
            link_size: u32::from_le_bytes(self.get_bytes::<4>()),
            link_off: u32::from_le_bytes(self.get_bytes::<4>()),
            map_off: u32::from_le_bytes(self.get_bytes::<4>()),
            string_ids_size: u32::from_le_bytes(self.get_bytes::<4>()),
            string_ids_off: u32::from_le_bytes(self.get_bytes::<4>()),
            type_ids_size: u32::from_le_bytes(self.get_bytes::<4>()),
            type_ids_off: u32::from_le_bytes(self.get_bytes::<4>()),
            proto_ids_size: u32::from_le_bytes(self.get_bytes::<4>()),
            proto_ids_off: u32::from_le_bytes(self.get_bytes::<4>()),
            field_ids_size: u32::from_le_bytes(self.get_bytes::<4>()),
            field_ids_off: u32::from_le_bytes(self.get_bytes::<4>()),
            method_ids_size: u32::from_le_bytes(self.get_bytes::<4>()),
            method_ids_off: u32::from_le_bytes(self.get_bytes::<4>()),
            class_defs_size: u32::from_le_bytes(self.get_bytes::<4>()),
            class_defs_off: u32::from_le_bytes(self.get_bytes::<4>()),
            data_size: u32::from_le_bytes(self.get_bytes::<4>()),
            data_off: u32::from_le_bytes(self.get_bytes::<4>()),
        }
    }
}