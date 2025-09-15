use crate::types::{Class_Def_Item, Field_Id_Item, Method_Id_Item, Proto_Id_Item};
use crate::utils::convert_vec_u8_to_vec_u32;

pub fn parse_ids_array(bytes: &mut Vec<u8>, length: u32) -> Vec<u32> {
    let mut id_bytes: Vec<u8> = bytes.drain(0..(length as usize * 4)).collect();
    convert_vec_u8_to_vec_u32(&mut id_bytes).expect("Vec<u8> to Vec<u32> conversion failed.")
}

pub fn parse_proto_id_array(bytes: &mut Vec<u8>, length: u32) -> Vec<Proto_Id_Item> {
    let mut return_vec: Vec<Proto_Id_Item> = Vec::new();
    for _ in 0..(length as usize) {
        let proto_id_item = Proto_Id_Item {
            shorty_idx: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            return_type_idx: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            parameters_off: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
        };
        return_vec.push(proto_id_item);
    }
    return return_vec;
}

pub fn parse_field_id_array(bytes: &mut Vec<u8>, length: u32) -> Vec<Field_Id_Item> {
    let mut return_vec: Vec<Field_Id_Item> = Vec::new();
    for _ in 0..(length as usize) {
        let field_id_item = Field_Id_Item {
            class_idx: u16::from_le_bytes(
                bytes.drain(0..2).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            type_idx: u16::from_le_bytes(
                bytes.drain(0..2).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            name_idx: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
        };
        return_vec.push(field_id_item);
    }
    return return_vec;
}

pub fn parse_method_id_array(bytes: &mut Vec<u8>, length: u32) -> Vec<Method_Id_Item> {
    let mut return_vec: Vec<Method_Id_Item> = Vec::new();
    for _ in 0..(length as usize) {
        let method_id_item = Method_Id_Item {
            class_idx: u16::from_le_bytes(
                bytes.drain(0..2).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            proto_idx: u16::from_le_bytes(
                bytes.drain(0..2).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            name_idx: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
        };
        return_vec.push(method_id_item);
    }
    return return_vec;
}

pub fn parse_class_defs_array(bytes: &mut Vec<u8>, length: u32) -> Vec<Class_Def_Item> {
    let mut return_vec: Vec<Class_Def_Item> = Vec::new();
    for _ in 0..(length as usize) {
        let class_def_item = Class_Def_Item {
            class_idx: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            access_flags: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            superclass_idx: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            interfaces_off: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            source_file_idx: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            annotations_off: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            class_data_off: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
            static_values_off: u32::from_le_bytes(
                bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap(),
            ),
        };
        return_vec.push(class_def_item);
    }
    return return_vec;
}
