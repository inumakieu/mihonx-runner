use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use crate::types::{DexClass, DexContainer};

pub fn save_class_to_file(class: &DexClass, path: &str) -> std::io::Result<()> {
    let full_path = "out/".to_owned() + path.rsplit_once(";").expect("Couldnt remove ;").0;
    let json_string = serde_json::to_string(class).expect("Failed to serialize to JSON");
    let dirs = full_path.rsplit_once('/').expect("Filepath is corrupted.");
    std::fs::create_dir_all(dirs.0).expect("Creating directories failed.");
    return std::fs::write(full_path, json_string);
}

pub fn save_container_to_file(container: &DexContainer, path: &str) -> std::io::Result<()> {
    let full_path = "out/".to_owned() + path;
    let json_string = serde_json::to_string(container).expect("Failed to serialize to JSON");
    return std::fs::write(full_path, json_string);
}

pub fn save_data_to_file(data: Vec<u8>, path: &str) -> std::io::Result<()> {
    let full_path = "out/".to_owned() + path;
    return std::fs::write(full_path, data);
}

pub fn class_file_to_class(path: &str) -> Option<DexClass> {
    let full_path = "out/".to_owned() + path.rsplit_once(";").expect("Couldnt remove ;").0;

    let file = File::open(full_path).expect("Opening file failed");
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let class: DexClass = serde_json::from_reader(reader).expect("Parsing content to class failed.");
    return Some(class)
}

pub fn save_strings_to_file(strings: Vec<String>, path: &str) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    for s in strings {
        writeln!(writer, "{}", s.replace("\n", "\\n"))?;
    }
    Ok(())
}

fn load_classes_from_dir(dir: &Path, classes: &mut Vec<DexClass>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectory
            load_classes_from_dir(&path, classes)?;
        } else if path.is_file() {
            // Try to open and parse as DexClass JSON
            if let Ok(file) = File::open(&path) {
                let reader = BufReader::new(file);
                if let Ok(class) = serde_json::from_reader::<_, DexClass>(reader) {
                    classes.push(class);
                }
            }
        }
    }
    Ok(())
}

pub fn load_classes_from_file() -> std::io::Result<Vec<DexClass>> {
    let mut classes = Vec::new();
    load_classes_from_dir(Path::new("out"), &mut classes)?;
    Ok(classes)
}

pub fn load_container_from_file(path: &str) -> std::io::Result<DexContainer> {
    let full_path = "out/".to_owned() + path;
    let file = File::open(full_path)?;
    let reader = BufReader::new(file);
    let container = serde_json::from_reader(reader).expect("Failed to deserialize DexContainer JSON");
    Ok(container)
}

pub fn load_data_from_file(path: &str) -> std::io::Result<Vec<u8>> {
    let full_path = "out/".to_owned() + path;
    std::fs::read(full_path)
}

pub fn load_strings_from_file(path: &str) -> std::io::Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut strings = Vec::new();
    for line in reader.lines() {
        strings.push(line?);
    }
    Ok(strings)
}

pub fn convert_vec_u8_to_vec_u16(data: &mut Vec<u8>) -> Result<Vec<u16>, &'static str> {
    Ok(data.chunks_exact(2).map(|c| u16::from_le_bytes(c.try_into().unwrap())).collect())
}

pub fn convert_vec_u8_to_vec_u32(data: &mut Vec<u8>) -> Result<Vec<u32>, &'static str> {
    if data.len() % 4 != 0 {
        return Err("Input Vec<u8> length must be a multiple of 4 for u32 conversion.");
    }
    Ok(data.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect())
}

pub fn parse_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

pub fn parse_i16(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([data[offset], data[offset + 1]])
}

pub fn parse_u32(data: &[u8], offset: usize) -> u32 {
    let available = (data.len() - 1) - offset;
    let mut slice: [u8; 4] = [0; 4];
    for i in 0..4 {
        if offset + i <= available {
            slice[i] = data[offset + i];
        } else {
            slice[i] = 0x00000000
        }
    }

    u32::from_le_bytes(slice)
}

pub fn parse_i32(data: &[u8], offset: usize) -> i32 {
    let available = (data.len() - 1) - offset;
    let mut slice: [u8; 4] = [0; 4];
    for i in 0..4 {
        if offset + i <= available {
            slice[i] = data[offset + i];
        } else {
            slice[i] = 0x00000000
        }
    }
    i32::from_le_bytes(slice)
}

pub fn parse_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3], data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]])
}

pub fn get_lower_bits(value: u8, num_bits: u8) -> u8 {
    let mask = (1 << num_bits) - 1; 

    value & mask
}