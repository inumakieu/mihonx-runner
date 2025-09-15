use crate::types::Header_Item;

pub fn parse_header_item(bytes: &mut Vec<u8>) -> Header_Item {
    Header_Item {
        magic: bytes.drain(0..8).collect::<Vec<u8>>().try_into().unwrap(),
        checksum: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        signature: bytes.drain(0..20).collect::<Vec<u8>>().try_into().unwrap(),
        file_size: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        header_size: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        endian_tag: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        link_size: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        link_off: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        map_off: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        string_ids_size: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        string_ids_off: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        type_ids_size: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        type_ids_off: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        proto_ids_size: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        proto_ids_off: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        field_ids_size: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        field_ids_off: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        method_ids_size: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        method_ids_off: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        class_defs_size: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        class_defs_off: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        data_size: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
        data_off: u32::from_le_bytes(bytes.drain(0..4).collect::<Vec<u8>>().try_into().unwrap()),
    }
}