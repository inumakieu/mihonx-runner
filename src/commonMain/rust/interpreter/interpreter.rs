use crate::parser::class::get_name_of_class;
use crate::parser::parser::Parser;
use crate::parser::strings::parse_string_at_offset;
use crate::{call_method, has_method, rust_log};
use crate::types::{DexClass, DexValue, Frame, Instruction, Object, ObjectId};
use crate::utils::class_file_to_class;
use std::collections::HashMap;
use jni::objects::GlobalRef;

#[macro_export]
macro_rules! interpreter_log {
    ($interpreter:expr, $($arg:tt)*) => {
        if $interpreter.parser.debug_enabled {
            println!($($arg)*);
        }
    };
}

pub struct Interpreter {
    pub parser: Parser, // owned parser, no lifetime parameter
    pub heap: HashMap<ObjectId, Object>,
    pub object_refs: Vec<GlobalRef>,
    pub frames: Vec<Frame>, // call stack
    pub main_idx: usize,
    pub next_object_id: ObjectId,
}

impl Interpreter {
    pub fn new(parser: Parser) -> Self {
        Self {
            parser,
            heap: HashMap::new(),
            object_refs: Vec::new(),
            frames: Vec::new(),
            main_idx: 0,
            next_object_id: 0,
        }
    }

    pub fn push_frame(&mut self, class_idx: usize, method_name: String, args: Vec<DexValue>) {
        let class = self.parser.classes[class_idx].clone();

        self.push_frame_with_class(&class, class_idx, method_name, args);
    }

    pub fn push_frame_with_class(&mut self, class: &DexClass, class_idx: usize, method_name: String, args: Vec<DexValue>) {
        
        let method_regs = {
            let method = class.methods.get(&method_name).expect("method not found");
            method.registers as usize
        };

        let mut registers = vec![DexValue::Null; method_regs];
        if args.len() > 0 {
            for i in 2..method_regs {
                registers[i] = args[i - 2].clone();
            }
        }

        self.frames.push(Frame {
            registers,
            temp: None,
            class_idx,
            method_name,
            pc: 0,
        });
    }

    pub fn pop_frame(&mut self) -> Option<Frame> {
        self.frames.pop()
    }

    pub fn alloc_object(&mut self, class_name: &str) -> ObjectId {
        let id = self.next_object_id;
        self.next_object_id += 1;
        self.heap.insert(
            id,
            Object {
                class_name: class_name.to_string(),
                fields: HashMap::new(),
                methods: HashMap::new()
            },
        );
        id
    }

    pub fn alloc_object_and_assign(&mut self, class_name: &str, dst: &u8) {
        let id = self.alloc_object(class_name);

        if let Some(frame) = self.frames.last_mut(){
            frame.registers[*dst as usize] = DexValue::Object(id);
        }
    }

    pub fn insert_object(&mut self, object: Object) -> ObjectId {
        let id = self.next_object_id;
        self.next_object_id += 1;
        self.heap.insert(
            id,
            object,
        );
        id
    }

    pub fn get_object(&mut self, class_name: &str) -> Option<usize> {
        if let Some((usize, object)) = self.heap.iter().find(|(id, object)| &object.class_name == class_name) {
            return Some(*usize)
        }

        return None
    }

    /// call a method by pointing to its owner class index and name
    pub fn call_method(
        &mut self,
        class_idx: usize,
        method_name: &str,
        args: Vec<DexValue>,
    ) -> Option<DexValue> {
        self.push_frame(class_idx, method_name.to_string(), args);
        self.run(class_idx)
    }

    /// Main execution loop. Returns an optional DexValue if top-level method returned a value.
    pub fn run(&mut self, class_idx: usize) -> Option<DexValue> {
        let class = self.parser.classes[class_idx].clone();
        
        self.run_with_class(&class, class_idx)
    }

    pub fn run_with_class(&mut self, class: &DexClass, class_idx: usize) -> Option<DexValue> {
        let mut return_value: Option<DexValue> = None;

        while let Some(frame) = self.frames.last_mut() {
            let method = class.methods
                .get(&frame.method_name);
            if let Some(method) = method {
                if frame.pc == 0 {
                    if frame.registers.len() < 2 {
                        frame.registers[0] = DexValue::Object(1);
                    } else {
                        frame.registers[1] = DexValue::Object(1);
                    }
                }
                if frame.pc >= method.instructions.len() {
                    self.pop_frame();
                    continue;
                }
                let instr = &method.instructions[frame.pc].clone();
                frame.pc += 1;
                     
                let value = self.execute(instr, class_idx);

                if let Some(value) = value {
                    return_value = Some(value);
                    self.pop_frame();
                    break
                }
            }
        }
        return_value
    }

    /// Execute one instruction with mutable access to interpreter (heap, frames, etc.)
    fn execute(&mut self, instr: &Instruction, class_idx: usize) -> Option<DexValue> {
        let frame = self.frames.last_mut().unwrap();

        match instr {
            Instruction::ConstString { dest, string_idx } => {
                interpreter_log!(self, "String_idx {:?}", &string_idx);
                let s = &self.parser.strings[*string_idx as usize];
                interpreter_log!(self, "Storing {:?} into v{}", &s, &dest);
                frame.registers[*dest as usize] = DexValue::String(s.to_string());
                interpreter_log!(self, "registers -> {:?}", &frame.registers);
            }

            Instruction::Const4Bit { dst, signed_int } => {
                interpreter_log!(self, "Signed int: {} -> v{}", signed_int, dst);
                frame.registers[*dst as usize] = DexValue::Int(*signed_int as i32);
            }

            Instruction::MoveObject { dst, src } => {
                interpreter_log!(self, "Moving Object {:?} from v{} -> v{}", frame.registers[*src as usize].clone(), src, dst);
                frame.registers[*dst as usize] = frame.registers[*src as usize].clone();
            }

            Instruction::MoveResultObject { dst } => {
                interpreter_log!(self, "Moving Result Object {:?} -> v{}", &frame.temp, dst);
                if let Some(temp) = &frame.temp {
                    frame.registers[*dst as usize] = temp.clone();
                    frame.temp = None;
                }
                interpreter_log!(self, "registers -> {:?}", &frame.registers);
            }

            Instruction::MoveResult { dst } => {
                if let Some(temp) = &frame.temp {
                    frame.registers[*dst as usize] = temp.clone();
                    frame.temp = None;
                }
            }

            Instruction::InvokeStatic {
                args, method_idx, argc
            } => {
                interpreter_log!(self, "Registers -> {:?}", &frame.registers);
                if args.len() < 2 {
                    return None
                }
                let first = &frame.registers[args[0] as usize];
                let second = &frame.registers[args[1] as usize];

                interpreter_log!(self, "First: {:?}, Second: {:?}", first, second);
                let method_name_idx = self.parser.container.clone().unwrap().method_id_to_string_offset(*method_idx as usize);

                if let Some(method_name_idx) = method_name_idx {
                    let method_name = parse_string_at_offset(&self.parser.data, method_name_idx, &self.parser.container.clone().unwrap().header_item, 0).1;
                    let class_name_idx = self.parser.container.clone().unwrap().method_id_to_class_string_offset(*method_idx as usize);
                    let class_name = get_name_of_class(class_name_idx, &self.parser.data, &self.parser.container.clone().unwrap().header_item, &self.parser.container.clone().unwrap().type_id_items, &self.parser.container.clone().unwrap().string_id_items);
                    interpreter_log!(self, "InvokeStatic -> {}{}", class_name, method_name);
                    
                    match method_name.as_str() {
                        "areEqual" => {
                            assert!(args.len() == 2, "areEqual requires 2 parameters");
                            let first = &frame.registers[args[0] as usize];
                            let second = &frame.registers[args[1] as usize];

                            interpreter_log!(self, "First: {:?}, Second: {:?}", first, second);
                            assert!(first == second, "Parameter {:?} is not equal to Parameter {:?}", first, second);
                            frame.temp = Some(DexValue::Boolean(first == second));

                        } 
                        "checkNotNullParameter" => {
                            let parameter = &frame.registers[args[1] as usize];
                            assert!(*parameter != DexValue::Null, "NullPointerException: parameter {:?} expected SOMETHING but found null", frame.registers[args[0] as usize]);
                        }
                        _ => {

                        }
                    }
                }
            }
            Instruction::InvokeSuper {
                args, method_idx, ..
            } => {
                let method_name_idx = self.parser.container.clone().unwrap().method_id_to_string_offset(*method_idx as usize);

                if let Some(method_name_idx) = method_name_idx {
                    let method_name = parse_string_at_offset(&self.parser.data, method_name_idx, &self.parser.container.clone().unwrap().header_item, 0).1;
                    let class_name_idx = self.parser.container.clone().unwrap().method_id_to_class_string_offset(*method_idx as usize);
                    let class_name = get_name_of_class(class_name_idx, &self.parser.data, &self.parser.container.clone().unwrap().header_item, &self.parser.container.clone().unwrap().type_id_items, &self.parser.container.clone().unwrap().string_id_items);
                    interpreter_log!(self, "InvokeSuper -> {}{}. Skipping for now.", class_name, method_name);
                }
            }
            Instruction::InvokeInterface {
                args, method_idx, ..
            } => {
                let method_name_idx = self.parser.container.clone().unwrap().method_id_to_string_offset(*method_idx as usize);

                if let Some(method_name_idx) = method_name_idx {
                    let method_name = parse_string_at_offset(&self.parser.data, method_name_idx, &self.parser.container.clone().unwrap().header_item, 0).1;
                    let parameters = "()";
                    let return_value = "Ljava/lang/String;";
                    let class_name_idx = self.parser.container.clone().unwrap().method_id_to_class_string_offset(*method_idx as usize);
                    let class_name = get_name_of_class(class_name_idx, &self.parser.data, &self.parser.container.clone().unwrap().header_item, &self.parser.container.clone().unwrap().type_id_items, &self.parser.container.clone().unwrap().string_id_items);
                    interpreter_log!(self, "Registers -> {:?}", &frame.registers);
                    interpreter_log!(self, "InvokeInterface -> {}{}.", class_name, method_name);

                    // find method in register objects
                    let mut object = self.heap.get(&1).unwrap().clone();
                    interpreter_log!(self, "Object -> {:?}", object);
                    for (_, field) in object.fields.clone() {
                        match field {
                            DexValue::Object(id) => {
                                interpreter_log!(self, "Object found. -> {}", id);
                                let ob = self.heap.get(&id).unwrap();
                                interpreter_log!(self, "Object -> {:?}", &ob);

                                if let Some(method) = ob.methods.get(&format!("{}:{}{}", &method_name, parameters, return_value)) {
                                    interpreter_log!(self, "Method found.");

                                    // check if GlobalRef exists with the needed method
                                    let signature = format!("{}{}{}", &method_name, &parameters, &return_value);
                                    for global_ref in self.object_refs.clone() {
                                        let obj = global_ref.as_obj(); // Get JObject
                                        
                                        if has_method(obj, &signature) {
                                            rust_log("Found it");
                                            let ret_value = call_method(obj, &method_name, &format!("{}{}", &parameters, &return_value).to_string(), &[]);
                                            interpreter_log!(self, "ret -> {:?}", &ret_value);
                                            frame.temp = Some(ret_value);
                                        }
                                    }

                                    if let Some(method) = method {
                                        let ret_value = method(&mut object, Vec::new());
                                        interpreter_log!(self, "Native Function -> {:?}", &ret_value);
                                        frame.temp = Some(ret_value);
                                        interpreter_log!(self, "registers -> {:?}", &frame.registers);
                                    }
                                }
                            }
                            _ => {

                            }
                        }
                    }
                }
            }
            Instruction::InvokeDirect {
                args, method_idx, ..
            } => {
                interpreter_log!(self, "Starting InvokeDirect");
                let method_name_idx = self.parser.container.clone().unwrap().method_id_to_string_offset(*method_idx as usize);

                if let Some(method_name_idx) = method_name_idx {
                    let method_name = parse_string_at_offset(&self.parser.data, method_name_idx, &self.parser.container.clone().unwrap().header_item, 0).1;
                    let class_name_idx = self.parser.container.clone().unwrap().method_id_to_class_string_offset(*method_idx as usize);
                    let class_name = get_name_of_class(class_name_idx, &self.parser.data, &self.parser.container.clone().unwrap().header_item, &self.parser.container.clone().unwrap().type_id_items, &self.parser.container.clone().unwrap().string_id_items);
                    
                    // let object_id = self.alloc_object(class_name.clone().as_str());

                    // out/{class_name}.json
                    interpreter_log!(self, "Loading {}", &class_name);
                    if class_name.contains("Ljava/lang/") {
                        return None
                    }
                    let loaded_class = class_file_to_class(&class_name);

                    if let Some(loaded_class) = loaded_class {
                        interpreter_log!(self, "Calling {}", &method_name);
                        let mut call_args = Vec::new();
                        for arg in args {
                            call_args.push(frame.registers[*arg as usize].clone());
                        }

                        self.push_frame_with_class(&loaded_class, class_name_idx, method_name, call_args);
                        let ret_value = self.run_with_class(&loaded_class, class_name_idx);
                        interpreter_log!(self, "Finished InvokeDirect");
                        return None;
                    }
                    
                }
            }
            Instruction::InvokeVirtual {
                args, method_idx, ..
            } => {
                let method_name_idx = self.parser.container.clone().unwrap().method_id_to_string_offset(*method_idx as usize);

                if let Some(method_name_idx) = method_name_idx {
                    let method_name = parse_string_at_offset(&self.parser.data, method_name_idx, &self.parser.container.clone().unwrap().header_item, 0).1;
                    let class_name_idx = self.parser.container.clone().unwrap().method_id_to_class_string_offset(*method_idx as usize);
                    let class_name = get_name_of_class(class_name_idx, &self.parser.data, &self.parser.container.clone().unwrap().header_item, &self.parser.container.clone().unwrap().type_id_items, &self.parser.container.clone().unwrap().string_id_items);
                    interpreter_log!(self, "InvokeVirtual -> {}{}. Skipping for now.", class_name, method_name);

                    // check if current class has method
                    match self.parser.classes[class_idx].methods.get(&class_name) {
                        Some(method) => {
                            interpreter_log!(self, "Method found in class. Executing now.");
                        }
                        None => {
                            interpreter_log!(self, "Method not found in class. Checking super class.");

                            let super_class = self.parser.classes[class_idx].super_class.clone().unwrap();
                            let loaded_class = class_file_to_class(&super_class);

                            let mut ret_value: Option<DexValue> = None;
                            if let Some(loaded_class) = loaded_class {
                                interpreter_log!(self, "Loaded class.");
                                interpreter_log!(self, "Calling {}", &method_name);
                                let mut call_args = Vec::new();
                                for arg in args {
                                    call_args.push(frame.registers[*arg as usize].clone());
                                }

                                self.push_frame_with_class(&loaded_class, class_name_idx, method_name, call_args);
                                ret_value = self.run_with_class(&loaded_class, class_name_idx);
                                interpreter_log!(self, "Finished InvokeDirect -> {:?}", ret_value);
                                return None;
                            }
                            frame.temp = ret_value.clone();
                        }
                    }
                }
            }

            Instruction::IPutObject {
                src,
                obj,
                instance_field_idx,
            } => {
                interpreter_log!(self, "Instance field -> {:02X}", instance_field_idx);
                interpreter_log!(self, "Registers -> {:?}", frame.registers);
                if let DexValue::Object(obj_id) = &frame.registers[*obj as usize] {

                    if let Some(object) = self.heap.get_mut(obj_id) {
                        let value = frame.registers[*src as usize].clone();
                        object
                            .fields
                            .insert(format!("field@{}", instance_field_idx), value.clone());
                        interpreter_log!(self, "IPutObject: Object {:?}, field@{} -> {:?}", object, instance_field_idx, value)
                    }
                }
            }

            Instruction::IPutBoolean {
                src,
                obj,
                instance_field_idx,
            } => {
                interpreter_log!(self, "Instance field -> {:02X}", instance_field_idx);
                if let DexValue::Object(obj_id) = &frame.registers[*obj as usize] {

                    if let Some(object) = self.heap.get_mut(obj_id) {
                        let value = frame.registers[*src as usize].clone();
                        object
                            .fields
                            .insert(format!("field@{}", instance_field_idx), value.clone().to_boolean().unwrap());
                        interpreter_log!(self, "IPutBoolean: Object {:?}, field@{} -> {:?}", object, instance_field_idx, value)
                    }
                }
            }

            Instruction::IGetObject { src, obj, instance_field_idx } => {
                if let DexValue::Object(obj_id) = &frame.registers[*obj as usize] {
                    if let Some(object) = self.heap.get_mut(obj_id) {
                        frame.registers[*src as usize] = object.fields[format!("field@{}", instance_field_idx).as_str()].clone();
                        interpreter_log!(self, "IGetObject: Object {:?}, field@{}", object, instance_field_idx)
                    }
                }
            }

            Instruction::IGetBoolean { src, obj, instance_field_idx } => {
                if let DexValue::Object(obj_id) = &frame.registers[*obj as usize] {
                    if let Some(object) = self.heap.get_mut(obj_id) {
                        interpreter_log!(self, "{:?}", object.fields);
                        frame.registers[*src as usize] = object.fields[format!("field@{}", instance_field_idx).as_str()].clone();
                        interpreter_log!(self, "IGetBoolean: Object {:?}, field@{}", object, instance_field_idx)
                    }
                }
            }

            Instruction::NewInstance { dst, type_idx } => {
                let string_idx = self.parser
                    .container
                    .clone()
                    .unwrap()
                    .type_to_string_id(*type_idx as usize)
                    .unwrap_or(0);
                
                if let Some(type_name) = self.parser.strings.get(string_idx) {
                    let type_name = type_name.clone();
                    interpreter_log!(self, "NewInstance: Type name -> {}", type_name);
                    self.alloc_object_and_assign(&type_name, dst);
                }
            }

            Instruction::CheckCast { ref_bearing_reg, type_idx } => {
                interpreter_log!(self, "Starting CheckCast. Registers -> {:?}", &frame.registers);
                let string_idx = self.parser.container.clone().unwrap().type_to_string_id(*type_idx as usize).unwrap_or(0);
                let type_name = self.parser.strings.get(string_idx);

                if let Some(type_name) = type_name {
                    // TODO: Do proper cast check
                    interpreter_log!(self, "CheckCast: Type name -> {}", type_name);
                }
            }

            Instruction::ReturnVoid => {
                interpreter_log!(self, "-----------------------------------------------------------------");
                interpreter_log!(self, "Registers -> {:?}", frame.registers);

                return Some(DexValue::Void)
            }

            Instruction::ReturnObject { src } => {
                interpreter_log!(self, "-----------------------------------------------------------------");
                interpreter_log!(self, "Registers -> {:?}", frame.registers);

                return Some(frame.registers[*src as usize].clone())
            }

            Instruction::Return { reg } => {
                interpreter_log!(self, "-----------------------------------------------------------------");
                interpreter_log!(self, "Registers -> {:?}", frame.registers);
                interpreter_log!(self, "Return Register: v{} -> {:?}", *reg, frame.registers);
                

                return Some(frame.registers[*reg as usize].clone())
            }

            _ => {
                interpreter_log!(self, "Unimplemented instruction: {:?}", instr);
            }
        }

        None
    }
}
