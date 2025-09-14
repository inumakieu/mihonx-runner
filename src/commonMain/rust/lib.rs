// The rust entry point of the mihonx-runner jni bindings

use jni::JNIEnv;
use jni::objects::{JObject, JString, JClass, JValue};
use jni::sys::jstring;
use jni::JavaVM;
use lazy_static::lazy_static;
use std::sync::Mutex;

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