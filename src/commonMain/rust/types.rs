
// Custom Representation
use serde::{Serialize, Deserialize};
use std::collections::HashMap;


/// Object identity in the heap
pub type ObjectId = usize;

pub type NativeMethod = fn(&mut Object, Vec<DexValue>) -> DexValue;

/// Representation of a heap object
#[derive(Debug, Clone)]
pub struct Object {
    pub class_name: String,
    pub fields: HashMap<String, DexValue>, // instance fields
    pub methods: HashMap<String, Option<NativeMethod>>, // e.g. "getUserAgent:()Ljava/lang/String;" -> fn
}

/// One method’s execution context
pub struct Frame {
    pub registers: Vec<DexValue>,
    pub temp: Option<DexValue>,
    /// Instead of borrowing a DexMethod, store an owned key to find it:
    pub class_idx: usize,
    pub method_name: String,
    pub pc: usize, // program counter (index into instructions)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    Nop,
    MoveObject { dst: u8, src: u8 },
    Move { dst: u8, src: u8 },
    Move16 { dst: u16, src: u16 },
    MoveWide { dst: u8, src: u8 },
    MoveWide16 { dst: u16, src: u16 },
    MoveWideFrom16 { dst: u8, src: u16 },
    MoveObjectFrom16 { dst: u8, src: u16 },
    MoveObject16 { dst: u16, src: u16 },
    MoveFrom16 { dst: u8, src: u16 },
    MoveException { dst: u8 },
    MoveResult { dst: u8 },
    MoveResultWide { dst: u8 },
    MoveResultObject { dst: u8 },
    ConstString { dest: u8, string_idx: u16 },
    ConstStringJumbo { dest: u8, string_idx: u32 },
    ReturnVoid,
    Return { reg: u8 },
    ReturnWide { reg: u8 },
    ReturnObject { src: u8 },
    Const4Bit { dst: u8, signed_int: i8 },
    Const16Bit { dst: u8, signed_int: i16 },
    Const32Bit { dst: u8, literal: u32 },
    ConstWide16Bit { dst: u8, signed_int: i16 },
    ConstWide16BitHigh { dst: u8, signed_int: i16 },
    ConstWide32 { dst: u8, literal: i32 },
    ConstWide64Bit { dst: u8, literal: u64 },
    ConstHigh16 { dst: u8, literal: i16 },
    Const { dst: u8, signed_int: u32 },
    ConstClass { dst: u8, type_idx: u16 },
    InstanceOf { dst: u8, ref_bearing_reg: u8, type_idx: u16 },

    MonitorEnter { ref_bearing_reg: u8 },
    MonitorExit { ref_bearing_reg: u8 },

    CheckCast { ref_bearing_reg: u8, type_idx: u16 },

    ArrayLength { dst: u8, array_ref_bearing_reg: u8 },
    NewInstance { dst: u8, type_idx: u16 },
    NewArray { dst: u8, size: u8, type_idx: u16 },
    FilledNewArray { argc: u8, args: Vec<u8>, type_idx: u16 },
    FilledNewArrayRange { count: u8, type_idx: u16, first_arg_reg: u16 },
    FilledArrayData { array_ref: u8, signed_fake_branch_off: i32 },

    Throw { reg: u8 },
    Goto { signed_branch_off: i8 },
    Goto16 { signed_branch_off: i16 },

    TestIfEqual { first_reg: u8, second_reg: u8, signed_branch_off: i16 },
    TestIfNotEqual { first_reg: u8, second_reg: u8, signed_branch_off: i16 },
    TestIfLessThan { first_reg: u8, second_reg: u8, signed_branch_off: i16 },
    TestIfGreaterEqual { first_reg: u8, second_reg: u8, signed_branch_off: i16 },
    TestIfGreaterThan { first_reg: u8, second_reg: u8, signed_branch_off: i16 },
    TestIfLessEqual { first_reg: u8, second_reg: u8, signed_branch_off: i16 },

    BranchIfEqualZero { test_reg: u8, signed_branch_off: i16 },
    BranchIfNotEqualZero { test_reg: u8, signed_branch_off: i16 },
    BranchIfLessThanZero { test_reg: u8, signed_branch_off: i16 },
    BranchIfGreaterEqualZero { test_reg: u8, signed_branch_off: i16 },
    BranchIfGreaterThanZero { test_reg: u8, signed_branch_off: i16 },
    BranchIfLessEqualZero { test_reg: u8, signed_branch_off: i16 },

    PackedSwitch { test_reg: u8, signed_fake_branch_off: i32 },
    SparseSwitch { test_reg: u8, signed_fake_branch_off: i32 },

    CmpLessFloat { dst: u8, first_reg: u8, second_reg: u8 },
    CmpGreaterFloat { dst: u8, first_reg: u8, second_reg: u8 },
    CmpLessDouble { dst: u8, first_reg: u8, second_reg: u8 },
    CmpGreaterDouble { dst: u8, first_reg: u8, second_reg: u8 },
    CmpLong { dst: u8, first_reg: u8, second_reg: u8 },

    AddInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },
    RSubInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },
    MulInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },
    DivInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },
    RemInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },
    AndInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },
    OrInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },
    XorInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },
    ShLInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },
    ShRInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },
    UShRInt8Lit8 { dst: u8, src: u8, signed_int_const: i8 },

    AddIntLit16 { dst: u8, src: u8, literal: i16 },
    RSubIntLit16 { dst: u8, src: u8, literal: i16 },
    MulIntLit16 { dst: u8, src: u8, literal: i16 },
    DivIntLit16 { dst: u8, src: u8, literal: i16 },
    RemIntLit16 { dst: u8, src: u8, literal: i16 },
    AndIntLit16 { dst: u8, src: u8, literal: i16 },
    OrIntLit16 { dst: u8, src: u8, literal: i16 },
    XorIntLit16 { dst: u8, src: u8, literal: i16 },

    SGet { src: u8, static_field_idx: u16 },
    SGetWide { src: u8, static_field_idx: u16 },
    SGetObject { src: u8, static_field_idx: u16 },
    SGetBoolean { src: u8, static_field_idx: u16 },
    SGetByte { src: u8, static_field_idx: u16 },
    SGetChar { src: u8, static_field_idx: u16 },
    SGetShort { src: u8, static_field_idx: u16 },
    SPut { src: u8, static_field_idx: u16 },
    SPutWide { src: u8, static_field_idx: u16 },
    SPutObject { src: u8, static_field_idx: u16 },
    SPutBoolean { src: u8, static_field_idx: u16 },
    SPutByte { src: u8, static_field_idx: u16 },
    SPutChar { src: u8, static_field_idx: u16 },
    SPutShort { src: u8, static_field_idx: u16 },

    AGet { src: u8, array_reg: u8, index_reg: u8 },
    AGetWide { src: u8, array_reg: u8, index_reg: u8 },
    AGetObject { src: u8, array_reg: u8, index_reg: u8 },
    AGetBoolean { src: u8, array_reg: u8, index_reg: u8 },
    AGetByte { src: u8, array_reg: u8, index_reg: u8 },
    AGetChar { src: u8, array_reg: u8, index_reg: u8 },
    AGetShort { src: u8, array_reg: u8, index_reg: u8 },
    APut { src: u8, array_reg: u8, index_reg: u8 },
    APutWide { src: u8, array_reg: u8, index_reg: u8 },
    APutObject { src: u8, array_reg: u8, index_reg: u8 },
    APutBoolean { src: u8, array_reg: u8, index_reg: u8 },
    APutByte { src: u8, array_reg: u8, index_reg: u8 },
    APutChar { src: u8, array_reg: u8, index_reg: u8 },
    APutShort { src: u8, array_reg: u8, index_reg: u8 },

    IGet { src: u8, obj: u8, instance_field_idx: u16 },
    IGetWide { src: u8, obj: u8, instance_field_idx: u16 },
    IGetObject { src: u8, obj: u8, instance_field_idx: u16 },
    IGetBoolean { src: u8, obj: u8, instance_field_idx: u16 },
    IGetByte { src: u8, obj: u8, instance_field_idx: u16 },
    IGetChar { src: u8, obj: u8, instance_field_idx: u16 },
    IGetShort { src: u8, obj: u8, instance_field_idx: u16 },
    IPut { src: u8, obj: u8, instance_field_idx: u16 },
    IPutWide { src: u8, obj: u8, instance_field_idx: u16 },
    IPutObject { src: u8, obj: u8, instance_field_idx: u16 },
    IPutBoolean { src: u8, obj: u8, instance_field_idx: u16 },
    IPutByte { src: u8, obj: u8, instance_field_idx: u16 },
    IPutChar { src: u8, obj: u8, instance_field_idx: u16 },
    IPutShort { src: u8, obj: u8, instance_field_idx: u16 },

    AddInt2Addr   { dst_and_first_src: u8, second_src: u8 },
    SubInt2Addr   { dst_and_first_src: u8, second_src: u8 },
    MulInt2Addr   { dst_and_first_src: u8, second_src: u8 },
    DivInt2Addr   { dst_and_first_src: u8, second_src: u8 },
    RemInt2Addr   { dst_and_first_src: u8, second_src: u8 },
    AndInt2Addr   { dst_and_first_src: u8, second_src: u8 },
    OrInt2Addr    { dst_and_first_src: u8, second_src: u8 },
    XorInt2Addr   { dst_and_first_src: u8, second_src: u8 },
    ShlInt2Addr   { dst_and_first_src: u8, second_src: u8 },
    ShrInt2Addr   { dst_and_first_src: u8, second_src: u8 },
    UshrInt2Addr  { dst_and_first_src: u8, second_src: u8 },

    AddLong2Addr  { dst_and_first_src: u8, second_src: u8 },
    SubLong2Addr  { dst_and_first_src: u8, second_src: u8 },
    MulLong2Addr  { dst_and_first_src: u8, second_src: u8 },
    DivLong2Addr  { dst_and_first_src: u8, second_src: u8 },
    RemLong2Addr  { dst_and_first_src: u8, second_src: u8 },
    AndLong2Addr  { dst_and_first_src: u8, second_src: u8 },
    OrLong2Addr   { dst_and_first_src: u8, second_src: u8 },
    XorLong2Addr  { dst_and_first_src: u8, second_src: u8 },
    ShlLong2Addr  { dst_and_first_src: u8, second_src: u8 },
    ShrLong2Addr  { dst_and_first_src: u8, second_src: u8 },
    UshrLong2Addr { dst_and_first_src: u8, second_src: u8 },

    AddFloat2Addr   { dst_and_first_src: u8, second_src: u8 },
    SubFloat2Addr   { dst_and_first_src: u8, second_src: u8 },
    MulFloat2Addr   { dst_and_first_src: u8, second_src: u8 },
    DivFloat2Addr   { dst_and_first_src: u8, second_src: u8 },
    RemFloat2Addr   { dst_and_first_src: u8, second_src: u8 },

    AddDouble2Addr  { dst_and_first_src: u8, second_src: u8 },
    SubDouble2Addr  { dst_and_first_src: u8, second_src: u8 },
    MulDouble2Addr  { dst_and_first_src: u8, second_src: u8 },
    DivDouble2Addr  { dst_and_first_src: u8, second_src: u8 },
    RemDouble2Addr  { dst_and_first_src: u8, second_src: u8 },

    AddInt { dst: u8, first_src: u8, second_src: u8 },
    SubInt { dst: u8, first_src: u8, second_src: u8 },
    MulInt { dst: u8, first_src: u8, second_src: u8 },
    DivInt { dst: u8, first_src: u8, second_src: u8 },
    RemInt { dst: u8, first_src: u8, second_src: u8 },
    AndInt { dst: u8, first_src: u8, second_src: u8 },
    OrInt { dst: u8, first_src: u8, second_src: u8 },
    XorInt { dst: u8, first_src: u8, second_src: u8 },
    ShLInt { dst: u8, first_src: u8, second_src: u8 },
    ShRInt { dst: u8, first_src: u8, second_src: u8 },
    UShRInt { dst: u8, first_src: u8, second_src: u8 },
    AddLong { dst: u8, first_src: u8, second_src: u8 },
    SubLong { dst: u8, first_src: u8, second_src: u8 },
    MulLong { dst: u8, first_src: u8, second_src: u8 },
    DivLong { dst: u8, first_src: u8, second_src: u8 },
    RemLong { dst: u8, first_src: u8, second_src: u8 },
    AndLong { dst: u8, first_src: u8, second_src: u8 },
    OrLong { dst: u8, first_src: u8, second_src: u8 },
    XorLong { dst: u8, first_src: u8, second_src: u8 },
    ShLLong { dst: u8, first_src: u8, second_src: u8 },
    ShRLong { dst: u8, first_src: u8, second_src: u8 },
    UShRLong { dst: u8, first_src: u8, second_src: u8 },
    AddFloat { dst: u8, first_src: u8, second_src: u8 },
    SubFloat { dst: u8, first_src: u8, second_src: u8 },
    MulFloat { dst: u8, first_src: u8, second_src: u8 },
    DivFloat { dst: u8, first_src: u8, second_src: u8 },
    RemFloat { dst: u8, first_src: u8, second_src: u8 },
    AddDouble { dst: u8, first_src: u8, second_src: u8 },
    SubDouble { dst: u8, first_src: u8, second_src: u8 },
    MulDouble { dst: u8, first_src: u8, second_src: u8 },
    DivDouble { dst: u8, first_src: u8, second_src: u8 },
    RemDouble { dst: u8, first_src: u8, second_src: u8 },

    NegInt { dst: u8, src: u8 },
    NotInt { dst: u8, src: u8 },
    NegLong { dst: u8, src: u8 },
    NotLong { dst: u8, src: u8 },
    NegFloat { dst: u8, src: u8 },
    NegDouble { dst: u8, src: u8 },
    IntToLong { dst: u8, src: u8 },
    IntToFloat { dst: u8, src: u8 },
    IntToDouble { dst: u8, src: u8 },
    LongToInt { dst: u8, src: u8 },
    LongToFloat { dst: u8, src: u8 },
    LongToDouble { dst: u8, src: u8 },
    FloatToInt { dst: u8, src: u8 },
    FloatToLong { dst: u8, src: u8 },
    FloatToDouble { dst: u8, src: u8 },
    DoubleToInt { dst: u8, src: u8 },
    DoubleToLong { dst: u8, src: u8 },
    DoubleToFloat { dst: u8, src: u8 },
    IntToByte { dst: u8, src: u8 },
    IntToChar { dst: u8, src: u8 },
    IntToShort { dst: u8, src: u8 },
    
    InvokeVirtualRange { count: u8, type_idx: u16, first_arg_reg: u16 },
    InvokeSuperRange { count: u8, type_idx: u16, first_arg_reg: u16 },
    InvokeDirectRange { count: u8, type_idx: u16, first_arg_reg: u16 },
    InvokeStaticRange { count: u8, type_idx: u16, first_arg_reg: u16 },
    InvokeInterfaceRange { count: u8, type_idx: u16, first_arg_reg: u16 },

    InvokeVirtual { argc: u8, args: Vec<u8>, method_idx: u16 },
    InvokeSuper { argc: u8, args: Vec<u8>, method_idx: u16 },
    InvokeDirect { argc: u8, args: Vec<u8>, method_idx: u16 },
    InvokeStatic { argc: u8, args: Vec<u8>, method_idx: u16 },
    InvokeInterface { argc: u8, args: Vec<u8>, method_idx: u16 },
    
    InvokeCustomRange { count: u8, call_site_ref: u16, first_arg_reg: u16 },
    ConstMethodHandle { dst: u8, method_handle_idx: u16 },
    ConstMethodType { dst: u8, method_proto_ref: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DexValue {
    Byte(i8),
    Short(i16),
    Char(u16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Boolean(bool),
    Null,
    String(String),
    Type(String),          // type_id → string representation
    Field(String),         // field_id → name
    Method(String),        // method_id → name
    MethodType(String),    // proto_id → description
    MethodHandle(u32),     // raw index for now
    Enum(String),          // field_id → name
    Array(Vec<DexValue>),  // encoded_array
    Annotation(Vec<(String, DexValue)>), // encoded_annotation as (name, value)
    Object(usize),
    KotlinObject(),
    Void
}

impl DexValue {
    pub fn to_boolean(&self) -> Option<DexValue> {
        match self {
            DexValue::Int(v) => Some(DexValue::Boolean(*v != 0)),
            _ => None, // unsupported conversion
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexField {
    pub ty: String,
    pub value: Option<DexValue>,
    pub is_static: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexMethod {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<String>,
    pub registers: u16,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexClass {
    pub name: String,
    pub super_class: Option<String>,
    pub static_fields: HashMap<String, DexField>,
    pub instance_fields: HashMap<String, DexField>,
    pub methods: HashMap<String, DexMethod>,
}


// Dex Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexContainer {
    pub header_item: Header_Item,

    /// Offsets (from start of data section) for each string_id entry.
    pub string_id_items: Vec<u32>,

    /// For each type_id, holds a string_id index.
    pub type_id_items: Vec<u32>,

    pub proto_id_items: Vec<Proto_Id_Item>,
    pub field_id_items: Vec<Field_Id_Item>,
    pub method_id_items: Vec<Method_Id_Item>,
    pub class_defs_items: Vec<Class_Def_Item>,
}

impl DexContainer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        header_item: Header_Item,
        string_id_items: Vec<u32>,
        type_id_items: Vec<u32>,
        proto_id_items: Vec<Proto_Id_Item>,
        field_id_items: Vec<Field_Id_Item>,
        method_id_items: Vec<Method_Id_Item>,
        class_defs_items: Vec<Class_Def_Item>,
    ) -> Self {
        Self {
            header_item,
            string_id_items,
            type_id_items,
            proto_id_items,
            field_id_items,
            method_id_items,
            class_defs_items,
        }
    }

    /// Returns (start, end) byte bounds for the data section within the whole DEX file.
    pub fn data_bounds(&self) -> (usize, usize) {
        let start = self.header_item.data_off as usize;
        let end = start + self.header_item.data_size as usize;
        (start, end)
    }

    /// Returns a slice pointing to the data section within the full file bytes.
    /// Panics if the header is inconsistent with the provided `whole_file`.
    pub fn data_slice<'a>(&self, whole_file: &'a [u8]) -> &'a [u8] {
        let (start, end) = self.data_bounds();
        &whole_file[start..end]
    }

    /// Safe lookup: string_id → string data offset (relative to data section).
    pub fn string_offset(&self, string_id: usize) -> Option<u32> {
        self.string_id_items.get(string_id).copied()
    }

    /// Safe lookup: type_id → string_id.
    pub fn type_to_string_id(&self, type_id: usize) -> Option<usize> {
        self.type_id_items.get(type_id).map(|&sid| sid as usize)
    }

    /// Convenience: type_id → string data offset (relative to data section).
    pub fn type_to_string_offset(&self, type_id: usize) -> Option<u32> {
        self.type_to_string_id(type_id)
            .and_then(|sid| self.string_offset(sid))
    }

    /// Convenience: method_id -> string data offset
    pub fn method_id_to_string_offset(&self, method_id: usize) -> Option<u32> {
        self.string_offset(
            self.method_id_items.get(method_id).unwrap().name_idx as usize
        )
    }

    /// Convenience: method_id -> class_id -> string data offset
    pub fn method_id_to_class_string_offset(&self, method_id: usize) -> usize {
        self.method_id_items.get(method_id).unwrap().class_idx as usize
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header_Item {
    pub magic: [u8; 8], // 3 bytes = "dex", 1 byte = \n, 3 bytes = Version Decimal, 1 byte = \0
    pub checksum: u32,
    pub signature: [u8; 20],
    pub file_size: u32,
    pub header_size: u32,
    pub endian_tag: u32, // ENDIAN_CONSTANT or REVERSE_ENDIAN_CONSTANT
    pub link_size: u32,
    pub link_off: u32,
    pub map_off: u32,
    pub string_ids_size: u32,
    pub string_ids_off: u32, // Useless as we drain the bytes
    pub type_ids_size: u32,
    pub type_ids_off: u32, // Useless as we drain the bytes
    pub proto_ids_size: u32,
    pub proto_ids_off: u32, // Useless as we drain the bytes
    pub field_ids_size: u32,
    pub field_ids_off: u32, // Useless as we drain the bytes
    pub method_ids_size: u32,
    pub method_ids_off: u32, // Useless as we drain the bytes
    pub class_defs_size: u32,
    pub class_defs_off: u32, // Useless as we drain the bytes
    pub data_size: u32,
    pub data_off: u32, // Useless as we drain the bytes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proto_Id_Item {
    pub shorty_idx: u32, // Index into string_ids
    pub return_type_idx: u32, // Index into type_ids
    pub parameters_off: u32 // Probably useless in Rust
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field_Id_Item {
    pub class_idx: u16,
    pub type_idx: u16,
    pub name_idx: u32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Method_Id_Item {
    pub class_idx: u16,
    pub proto_idx: u16,
    pub name_idx: u32
}

pub static NO_INDEX: u32 = 0xffffffff;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class_Def_Item {
    // Index into type_ids
    pub class_idx: u32,
    // Access flags for the class (public, private, ...)
    pub access_flags: u32, 
    // Index into type_ids for superclass, or NO_INDEX
    pub superclass_idx: u32,
    // Offset into data, or 0
    pub interfaces_off: u32, 
    // Index into strings_id or NO_INDEX
    pub source_file_idx: u32,
    // Offset into data, or 0
    pub annotations_off: u32,
    // Offset into data, or 0
    pub class_data_off: u32,
    // Offset into data, or 0
    pub static_values_off: u32, 
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeItem {
    pub registers_size: u16,
    pub ins_size: u16,
    pub outs_size: u16,
    pub tries_size: u16,
    pub debug_info_off: u32,
    pub insns_size: u32,
    pub insns: Vec<u8>,
    pub instructions: Vec<Instruction>,
    pub padding: Option<u16>,
}