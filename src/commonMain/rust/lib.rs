// The rust entry point of the mihonx-runner jni bindings

use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JObject, JString, JValue};
use jni::sys::jstring;
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
        
        rust_log("Log from rust.");

        return result.into_raw();
    }
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

    let parser = Parser::initialize_from_files();
    let mut interpreter = Interpreter::new(parser);

    let global_ctx = env.new_global_ref(ctx).unwrap();
    interpreter.object_refs.push(global_ctx);

    let main_idx = &interpreter.parser
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

    // Ensure <init> exists
    if !interpreter.parser.classes[*main_idx].methods.contains_key("<init>") {
        panic!("No <init> method found for main class");
    }

    let mut ctx = Object {
        class_name: "mihonx.ExtensionContextImpl".to_string(),
        fields: HashMap::new(),
        methods: HashMap::new(),
    };

    ctx.methods.insert(
        "getUserAgent:()Ljava/lang/String;".to_string(),
        None,
    );

    let extension_context = interpreter.insert_object(ctx);

    let args: Vec<DexValue> = vec![
        DexValue::Object(extension_context)
    ];

    // pass class index and method name; no borrow to parser remains across the call
    
    let class_name = &interpreter.parser.classes[*main_idx].name.clone();
    interpreter.alloc_object(&class_name);

    interpreter.call_method(*main_idx, "<init>", args).unwrap();
    let name= interpreter.call_method(*main_idx, "getName", Vec::new()).unwrap();

    match name {
        DexValue::String(string) => {
            rust_log(&string);
            return env.new_string(string).unwrap().into_raw();
        }
        _ => {
            return env.new_string("Data could not be returned.").unwrap().into_raw();
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