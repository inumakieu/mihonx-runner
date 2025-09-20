// The rust entry point of the mihonx-runner jni bindings

use jni::JNIEnv;
use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jstring};
use jni::JavaVM;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::parser::parser::Parser;
use crate::interpreter::interpreter::Interpreter;
use crate::types::{DexValue, Object};

mod parser;
mod utils;
mod types;
mod interpreter;

lazy_static! {
    static ref JVM: Mutex<Option<JavaVM>> = Mutex::new(None);
    static ref INTERPRETER: Mutex<Option<Interpreter>> = Mutex::new(None);
}

/// Initialize interpreter if not already initialized
fn get_or_init_interpreter() -> std::sync::MutexGuard<'static, Option<Interpreter>> {
    let mut guard = INTERPRETER.lock().unwrap();
    if guard.is_none() {
        let parser = Parser::initialize_from_files();
        *guard = Some(Interpreter::new(parser));
    }
    guard
}

pub fn call_method(
    obj: &JObject,
    method_name: &str,
    signature: &str,
    args: &[JValue],
) -> DexValue {
    let vm_guard = JVM.lock().unwrap();
    let vm = vm_guard.as_ref().unwrap(); // JavaVM
    let mut env = vm.attach_current_thread().expect("Failed to attach thread");
    
    let ret_value = env.call_method(obj, method_name, signature, args);

    if let Ok(ret_value) = ret_value {
        match ret_value {
            jni::objects::JValueGen::Bool(value) => {
                return DexValue::Boolean(value != 0)
            },
            jni::objects::JValueGen::Int(value) => {
                return DexValue::Boolean(value != 0)
            }
            jni::objects::JValueGen::Object(obj) => {
                let string_class = env.find_class("java/lang/String").unwrap();
                if !env.is_instance_of(&obj, string_class).unwrap() {
                    return DexValue::Null
                }

                // Cast JObject to JString
                let jstring = obj.into();

                // Get the Java String content and convert to Rust String
                let rust_string = env.get_string(&jstring).unwrap().into();

                return DexValue::String(rust_string);
            }
            _ => {
                rust_log("unknown");
                return DexValue::Null
            }
        }
    }

    return DexValue::Null
}

pub fn has_method(obj: &JObject, sig: &str) -> bool {
    let vm_guard = JVM.lock().unwrap();
    let vm = vm_guard.as_ref().unwrap(); // JavaVM
    let mut env = vm.attach_current_thread().expect("Failed to attach thread");
    // Split "getUserAgent()Ljava/lang/String;" into name + signature
    let parts: Vec<&str> = sig.splitn(2, '(').collect();
    if parts.len() != 2 {
        return false;
    }
    let name = parts[0];
    let desc = format!("({}", parts[1]); // add back "("

    // Get the class of the object
    let cls = match env.get_object_class(obj) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Try resolving the method
    env.get_method_id(cls, name, &desc).is_ok()
}

/// Called once at startup from Kotlin's init block
#[unsafe(no_mangle)]
pub extern "system" fn Java_mihonx_runner_RustBridge_nativeInit(
    env: JNIEnv,
    _class: JClass,
) {
    let vm = env.get_java_vm().expect("Failed to get JavaVM");
    *JVM.lock().unwrap() = Some(vm);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_mihonx_runner_RustBridge_rustUseExtensionContext(
    env: JNIEnv,
    _this: JObject,
    ctx: JObject,
) -> jstring {
    unsafe {
        // Call ctx.getUserAgent()
        let result = env.unsafe_clone()
            .call_method(&ctx, "getUserAgent", "()Ljava/lang/String;", &[])
            .expect("failed call")
            .l()
            .unwrap();
        
        return result.into_raw();
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_mihonx_runner_RustBridge_rustGetDexVersion(
    env: JNIEnv,
    _this: JObject
) -> jstring {
    let mut guard = get_or_init_interpreter();
    let interpreter = guard.as_mut().unwrap();

    return env.new_string(format!("v{:?}", String::from_utf8(interpreter.parser.container.clone().unwrap().header_item.magic[4..6].into()))).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_mihonx_runner_RustBridge_rustInstallExtension(
    env: JNIEnv,
    _this: JObject,
    bytes: JByteArray
) {
    rust_log("Installing extension.");

    let array_len = env.get_array_length(&bytes).unwrap();
    let mut rust_bytes = vec![0; array_len as usize];
    env.get_byte_array_region(&bytes, 0, &mut rust_bytes)
        .unwrap();

    let mut parser = Parser::new(rust_bytes.into_iter().map(|x| x as u8).collect(), true);
    parser.parse();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_mihonx_runner_RustBridge_rustExtensionGetName(
    env: JNIEnv,
    _this: JObject,
    ctx: JObject,
) -> jstring {
    rust_log("Running init and getName");

    let mut guard = get_or_init_interpreter();
    let interpreter = guard.as_mut().unwrap();

    let global_ctx = env.new_global_ref(ctx).unwrap();
    interpreter.object_refs.push(global_ctx);

    let main_idx = interpreter
        .parser
        .classes
        .iter()
        .position(|class| {
            class
                .super_class
                .as_ref()
                .map(|sc| sc.contains("Source"))
                .unwrap_or(false)
        })
        .expect("No Main Class found.");

    if !interpreter.parser.classes[main_idx].methods.contains_key("<init>") {
        panic!("No <init> method found for main class");
    }

    interpreter.main_idx = main_idx;

    let mut ctx = Object {
        class_name: "mihonx.ExtensionContextImpl".to_string(),
        fields: HashMap::new(),
        methods: HashMap::new(),
    };

    ctx.methods
        .insert("getUserAgent:()Ljava/lang/String;".to_string(), None);

    let extension_context = interpreter.insert_object(ctx);
    let args = vec![DexValue::Object(extension_context)];

    let class_name = interpreter.parser.classes[main_idx].name.clone();
    interpreter.alloc_object(&class_name);

    interpreter.call_method(main_idx, "<init>", args).unwrap();
    let name = interpreter.call_method(main_idx, "getName", Vec::new()).unwrap();

    match name {
        DexValue::String(string) => {
            rust_log(&string);
            env.new_string(string).unwrap().into_raw()
        }
        _ => env.new_string("Data could not be returned.").unwrap().into_raw(),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_mihonx_runner_RustBridge_rustExtensionIsUserAgentEqual(
    env: JNIEnv,
    _this: JObject
) -> jboolean {
    let mut guard = get_or_init_interpreter();
    let interpreter = guard.as_mut().unwrap();

    let isCorrectUserAgent= interpreter.call_method(interpreter.main_idx, "isCorrectUserAgent", Vec::new()).unwrap();

    match isCorrectUserAgent {
        DexValue::Boolean(value) => {
            return value as jni::sys::jboolean;
        }
        _ => {
            return false as jni::sys::jboolean;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_mihonx_runner_RustBridge_rustExtensionCallMethod(
    mut env: JNIEnv,
    _this: JObject,
    method_name: JString,
) {
    let mut guard = get_or_init_interpreter();
    let interpreter = guard.as_mut().unwrap();

    let jstring_obj = JString::from(method_name);

    // 2. Get JavaStr from JNIEnv
    let java_str = env.get_string(&jstring_obj);

    // 3. Convert JavaStr to Rust String
    let rust_string: String = java_str.unwrap().into();

    let returnValue= interpreter.call_method(interpreter.main_idx, &rust_string, vec![]).unwrap();

    match returnValue {
        DexValue::Object(id) => {
            println!("{:?}", interpreter.heap[&id])
        }
        _ => {
            let message = format!("{:?}", returnValue);

            rust_log(&message);
        }
    }
}

/// Rust function you can call anywhere to log via Kotlin
pub fn rust_log(msg: &str) {
    let vm_mutex = JVM.lock();
    let vm = vm_mutex.as_ref().unwrap().as_ref().unwrap();
    let mut env = vm.attach_current_thread().expect("attach failed");

    let jmsg = env.new_string(msg).expect("Couldn't create java string");

    // Convert JString -> JObject, then wrap in JValue::Object
    let obj = JObject::from(jmsg);
    let arg: JValue = JValue::Object(&obj);

    let bridge_class = env
        .find_class("mihonx/runner/RustBridge")
        .expect("Class not found");

    let _ = env.call_static_method(
        bridge_class,
        "logFromRust",
        "(Ljava/lang/String;)V",
        &[arg],
    );
}