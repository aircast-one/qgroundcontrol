use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::OnceLock;

use jni::objects::{GlobalRef, JClass, JObject, JObjectArray, JString, JValue};
use jni::sys::{JNI_VERSION_1_6, jboolean, jfloat, jint, jlong, jstring};
use jni::{JNIEnv, JavaVM};

const BRIDGE_CLASS: &str = "org/mavlink/qgroundcontrol/QGCBridge";

static VM: OnceLock<JavaVM> = OnceLock::new();
static BRIDGE: OnceLock<GlobalRef> = OnceLock::new();

fn text_of(env: &mut JNIEnv, value: &JString) -> CString {
    let read = match value.is_null() {
        true => String::new(),
        false => env.get_string(value).map(String::from).unwrap_or_default(),
    };
    CString::new(read.replace('\0', "")).unwrap_or_default()
}

fn answered(env: &mut JNIEnv, raw: *mut c_char) -> jstring {
    let read = match raw.is_null() {
        true => String::new(),
        false => unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned(),
    };
    unsafe { crate::nativehost::qgc_bridge_free(raw) };
    env.new_string(read).map(JString::into_raw).unwrap_or(std::ptr::null_mut())
}

unsafe extern "C" fn relay(path: *const c_char, json: *const c_char) {
    let (Some(vm), Some(bridge)) = (VM.get(), BRIDGE.get()) else { return };
    let read = |raw: *const c_char| if raw.is_null() { String::new() } else { unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned() };
    let (path, json) = (read(path), read(json));
    let Ok(mut env) = vm.attach_current_thread_as_daemon() else { return };
    let _ = env.with_local_frame(4, |env| -> jni::errors::Result<()> {
        let class: &JClass = bridge.as_obj().into();
        let (path, json) = (env.new_string(path)?, env.new_string(json)?);
        env.call_static_method(class, "onEvent", "(Ljava/lang/String;Ljava/lang/String;)V", &[JValue::Object(&path), JValue::Object(&json)])?;
        Ok(())
    });
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *mut c_void) -> jint {
    let Ok(mut env) = vm.get_env() else { return JNI_VERSION_1_6 };
    if let Some(bridge) = env.find_class(BRIDGE_CLASS).ok().and_then(|class| env.new_global_ref(class).ok()) {
        let _ = BRIDGE.set(bridge);
    }
    let _ = VM.set(vm);
    unsafe { crate::nativehost::qgc_bridge_set_event_handler(Some(relay)) };
    JNI_VERSION_1_6
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_start(mut env: JNIEnv, _class: JClass, arguments: JObjectArray) -> jint {
    let count = if arguments.is_null() { 0 } else { env.get_array_length(&arguments).unwrap_or(0) };
    let given: Vec<CString> = (0..count)
        .filter_map(|i| {
            let element = env.get_object_array_element(&arguments, i).ok()?;
            Some(text_of(&mut env, &JString::from(element)))
        })
        .collect();
    let argv: Vec<CString> = std::iter::once(CString::new("aircast").unwrap_or_default()).chain(given).collect();
    let pointers: Vec<*const c_char> = argv.iter().map(|a| a.as_ptr()).collect();
    unsafe { crate::nativehost::qgc_start(pointers.len() as jint, pointers.as_ptr()) }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_shutdown(_env: JNIEnv, _class: JClass) {
    crate::nativehost::qgc_shutdown();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_get(mut env: JNIEnv, _class: JClass, path: JString) -> jstring {
    let path = text_of(&mut env, &path);
    let raw = unsafe { crate::nativehost::qgc_bridge_get(path.as_ptr()) };
    answered(&mut env, raw)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_getFields(mut env: JNIEnv, _class: JClass, path: JString, fields: JString) -> jstring {
    let (path, fields) = (text_of(&mut env, &path), text_of(&mut env, &fields));
    let raw = unsafe { crate::nativehost::qgc_bridge_get_fields(path.as_ptr(), fields.as_ptr()) };
    answered(&mut env, raw)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_set(mut env: JNIEnv, _class: JClass, path: JString, json: JString) -> jstring {
    let (path, json) = (text_of(&mut env, &path), text_of(&mut env, &json));
    let raw = unsafe { crate::nativehost::qgc_bridge_set(path.as_ptr(), json.as_ptr()) };
    answered(&mut env, raw)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_invoke(mut env: JNIEnv, _class: JClass, path: JString, args: JString) -> jstring {
    let (path, args) = (text_of(&mut env, &path), text_of(&mut env, &args));
    let raw = unsafe { crate::nativehost::qgc_bridge_invoke(path.as_ptr(), args.as_ptr()) };
    answered(&mut env, raw)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_coreLinkOpen(mut env: JNIEnv, _class: JClass, config: JString) -> jstring {
    let config = text_of(&mut env, &config);
    let raw = unsafe { crate::abi::qgc_core_link_open(config.as_ptr()) };
    answered(&mut env, raw)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_nativeWatch(mut env: JNIEnv, _class: JClass, paths: JString) {
    let paths = text_of(&mut env, &paths);
    unsafe { crate::nativehost::qgc_bridge_watch(paths.as_ptr()) };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_notifyDeepLink(mut env: JNIEnv, _class: JClass, url: JString) {
    let url = text_of(&mut env, &url);
    unsafe { crate::nativehost::qgc_handle_deep_link(url.as_ptr()) };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_notifyFontScale(_env: JNIEnv, _class: JClass, _scale: jfloat) {}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_notifySafeAreaInsets(_env: JNIEnv, _class: JClass, _left: jint, _top: jint, _right: jint, _bottom: jint) {}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_videoWidth(_env: JNIEnv, _class: JClass) -> jint {
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_videoHeight(_env: JNIEnv, _class: JClass) -> jint {
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_videoFrames(_env: JNIEnv, _class: JClass) -> jlong {
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_videoCopyFrame(_env: JNIEnv, _class: JClass, _buffer: JObject) -> jboolean {
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_videoSetSurface(_env: JNIEnv, _class: JClass, _surface: JObject) -> jboolean {
    0
}
