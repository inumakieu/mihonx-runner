pub fn read_uleb128(data: &[u8], mut offset: usize) -> (u32, usize) {
    let mut result = 0u32;
    let mut shift = 0;
    loop {
        let byte = data[offset];
        offset += 1;
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    (result, offset)
}
