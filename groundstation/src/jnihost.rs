use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::{Mutex, OnceLock, PoisonError};

use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JObjectArray, JString, JValue, JValueOwned};
use jni::sys::{JNI_VERSION_1_6, jboolean, jbyteArray, jfloat, jint, jlong, jstring};
use jni::{JNIEnv, JavaVM};

const BRIDGE_CLASS: &str = "org/mavlink/qgroundcontrol/QGCBridge";
const USB_SERIAL_CLASS: &str = "org/mavlink/qgroundcontrol/QGCUsbSerialManager";
const BLUETOOTH_CLASS: &str = "one/aircast/android/BluetoothLinks";
const WFB_USB_CLASS: &str = "one/aircast/android/WfbUsb";
const USB_DISKS_CLASS: &str = "one/aircast/android/UsbDisks";
const WFB_LIBRARY: &CStr = c"libqgc_wfb.so";
const RTLD_NOW: std::ffi::c_int = 2;
const USB_WRITE_TIMEOUT_MS: i32 = 1000;
const BAD_DEVICE_ID: i32 = 0;

static VM: OnceLock<JavaVM> = OnceLock::new();
static BRIDGE: OnceLock<GlobalRef> = OnceLock::new();
static USB_SERIAL: OnceLock<GlobalRef> = OnceLock::new();
static BLUETOOTH: OnceLock<GlobalRef> = OnceLock::new();
static WFB_USB: OnceLock<GlobalRef> = OnceLock::new();
static USB_DISKS: OnceLock<GlobalRef> = OnceLock::new();

unsafe extern "C" {
    fn dlopen(name: *const c_char, flags: std::ffi::c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}
static USB_DEVICES: Mutex<BTreeMap<u32, i32>> = Mutex::new(BTreeMap::new());

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

enum Arg<'a> {
    Int(i32),
    Long(i64),
    Text(&'a str),
    Bytes(&'a [u8]),
}

fn usb_call(name: &str, signature: &str, arguments: &[Arg]) -> Option<JValueOwned<'static>> {
    static_call(USB_SERIAL.get()?, name, signature, arguments)
}

fn bluetooth_call(name: &str, signature: &str, arguments: &[Arg]) -> Option<JValueOwned<'static>> {
    static_call(BLUETOOTH.get()?, name, signature, arguments)
}

fn static_call(class: &GlobalRef, name: &str, signature: &str, arguments: &[Arg]) -> Option<JValueOwned<'static>> {
    let vm = VM.get()?;
    let mut env = vm.attach_current_thread_as_daemon().ok()?;
    let answer = env
        .with_local_frame(8, |env| -> jni::errors::Result<JValueOwned<'static>> {
            let objects: Vec<Option<JObject>> = arguments
                .iter()
                .map(|argument| match argument {
                    Arg::Text(text) => env.new_string(text).map(|value| Some(value.into())),
                    Arg::Bytes(bytes) => env.byte_array_from_slice(bytes).map(|value| Some(value.into())),
                    _ => Ok(None),
                })
                .collect::<jni::errors::Result<_>>()?;
            let null = JObject::null();
            let given: Vec<JValue> = arguments
                .iter()
                .zip(&objects)
                .map(|(argument, object)| match (argument, object) {
                    (Arg::Int(value), _) => JValue::Int(*value),
                    (Arg::Long(value), _) => JValue::Long(*value),
                    (_, Some(object)) => JValue::Object(object),
                    (_, None) => JValue::Object(&null),
                })
                .collect();
            let class: &JClass = class.as_obj().into();
            Ok(match env.call_static_method(class, name, signature, &given)? {
                JValueOwned::Int(value) => JValueOwned::Int(value),
                JValueOwned::Bool(value) => JValueOwned::Bool(value),
                _ => JValueOwned::Void,
            })
        })
        .ok();
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    answer
}

fn usb_device(id: u32) -> Option<i32> {
    USB_DEVICES.lock().unwrap_or_else(PoisonError::into_inner).get(&id).copied()
}

fn usb_open(id: u32, name: &str, baud: u32, data_bits: i64, stop_bits: i64, parity: i64) -> bool {
    let opened = usb_call("open", "(Ljava/lang/String;J)I", &[Arg::Text(name), Arg::Long(i64::from(id))])
        .and_then(|value| value.i().ok())
        .filter(|device| *device != BAD_DEVICE_ID);
    let Some(device) = opened else { return false };
    USB_DEVICES.lock().unwrap_or_else(PoisonError::into_inner).insert(id, device);
    let parity = crate::platformserial::android_parity(parity);
    let configured = usb_call("setParameters", "(IIIII)Z", &[Arg::Int(device), Arg::Int(baud as i32), Arg::Int(data_bits as i32), Arg::Int(stop_bits as i32), Arg::Int(parity as i32)])
        .and_then(|value| value.z().ok())
        .unwrap_or(false);
    let started = configured && usb_call("startIoManager", "(I)Z", &[Arg::Int(device)]).and_then(|value| value.z().ok()).unwrap_or(false);
    if !started {
        usb_close(id);
    }
    started
}

fn usb_write(id: u32, bytes: &[u8]) -> bool {
    let Some(device) = usb_device(id) else { return false };
    let length = bytes.len() as i32;
    usb_call("write", "(I[BII)I", &[Arg::Int(device), Arg::Bytes(bytes), Arg::Int(length), Arg::Int(USB_WRITE_TIMEOUT_MS)])
        .and_then(|value| value.i().ok())
        .is_some_and(|written| written == length)
}

fn usb_close(id: u32) {
    let Some(device) = USB_DEVICES.lock().unwrap_or_else(PoisonError::into_inner).remove(&id) else { return };
    usb_call("stopIoManager", "(I)Z", &[Arg::Int(device)]);
    usb_call("close", "(I)Z", &[Arg::Int(device)]);
}

fn bluetooth_open(id: u32, address: &str) -> bool {
    bluetooth_call("open", "(Ljava/lang/String;J)Z", &[Arg::Text(address), Arg::Long(i64::from(id))]).and_then(|value| value.z().ok()).unwrap_or(false)
}

fn bluetooth_write(id: u32, bytes: &[u8]) -> bool {
    bluetooth_call("write", "(J[B)Z", &[Arg::Long(i64::from(id)), Arg::Bytes(bytes)]).and_then(|value| value.z().ok()).unwrap_or(false)
}

fn bluetooth_close(id: u32) {
    bluetooth_call("close", "(J)V", &[Arg::Long(i64::from(id))]);
}

fn bluetooth_scan(start: bool) -> bool {
    bluetooth_call("scan", "(I)Z", &[Arg::Int(i32::from(start))]).and_then(|value| value.z().ok()).unwrap_or(false)
}

fn bluetooth_scanning() -> bool {
    bluetooth_call("scanning", "()Z", &[]).and_then(|value| value.z().ok()).unwrap_or(false)
}

fn bluetooth_devices() -> Vec<String> {
    let (Some(vm), Some(class)) = (VM.get(), BLUETOOTH.get()) else { return Vec::new() };
    string_array(vm, class, "devicesInfo")
}

fn string_array(vm: &JavaVM, class: &GlobalRef, method: &str) -> Vec<String> {
    let Ok(mut env) = vm.attach_current_thread_as_daemon() else { return Vec::new() };
    let lines = env
        .with_local_frame(8, |env| -> jni::errors::Result<Vec<String>> {
            let class: &JClass = class.as_obj().into();
            let array = JObjectArray::from(env.call_static_method(class, method, "()[Ljava/lang/String;", &[])?.l()?);
            let count = if array.is_null() { 0 } else { env.get_array_length(&array)? };
            Ok((0..count)
                .filter_map(|i| {
                    let element = env.get_object_array_element(&array, i).ok()?;
                    env.get_string(&JString::from(element)).ok().map(String::from)
                })
                .collect())
        })
        .unwrap_or_default();
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    lines
}

fn wfb_devices() -> Vec<crate::wfbhost::UsbDevice> {
    let (Some(vm), Some(class)) = (VM.get(), WFB_USB.get()) else { return Vec::new() };
    string_array(vm, class, "devices")
        .iter()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            Some(crate::wfbhost::UsbDevice {
                name: fields.first()?.to_string(),
                vendor_id: fields.get(1)?.parse().ok()?,
                product_id: fields.get(2)?.parse().ok()?,
                product: fields.get(3).map(|p| p.to_string()).unwrap_or_default(),
            })
        })
        .collect()
}

fn wfb_open(name: &str) -> i32 {
    WFB_USB.get().and_then(|class| static_call(class, "open", "(Ljava/lang/String;)I", &[Arg::Text(name)])).and_then(|value| value.i().ok()).unwrap_or(-1)
}

fn wfb_close(name: &str) {
    if let Some(class) = WFB_USB.get() {
        static_call(class, "close", "(Ljava/lang/String;)V", &[Arg::Text(name)]);
    }
}

fn wfb_symbol<T>(handle: *mut c_void, name: &CStr) -> Option<T> {
    let found = unsafe { dlsym(handle, name.as_ptr()) };
    (!found.is_null()).then(|| unsafe { std::mem::transmute_copy(&found) })
}

fn wfb_library() -> Option<crate::wfbhost::FdLibrary> {
    let handle = unsafe { dlopen(WFB_LIBRARY.as_ptr(), RTLD_NOW) };
    (!handle.is_null()).then_some(())?;
    let table: unsafe extern "C" fn() -> *const crate::wfbhost::Native = wfb_symbol(handle, c"qgc_wfb_native")?;
    let native = unsafe { table().as_ref() }?;
    Some(crate::wfbhost::FdLibrary {
        native: crate::wfbhost::Native { ..*native },
        start_fd: wfb_symbol(handle, c"qgc_wfb_start_fd")?,
        adapter_name: wfb_symbol(handle, c"qgc_wfb_adapter_name")?,
    })
}

fn disk_list() -> Vec<crate::platformdisk::Disk> {
    let (Some(vm), Some(class)) = (VM.get(), USB_DISKS.get()) else { return Vec::new() };
    string_array(vm, class, "devices").iter().filter_map(|line| line.split_once('\t')).map(|(id, name)| crate::platformdisk::Disk { id: id.to_string(), name: name.to_string() }).collect()
}

fn disk_open(id: &str) -> i32 {
    USB_DISKS.get().and_then(|class| static_call(class, "open", "(Ljava/lang/String;)I", &[Arg::Text(id)])).and_then(|value| value.i().ok()).unwrap_or(-1)
}

fn disk_close(id: &str) {
    if let Some(class) = USB_DISKS.get() {
        static_call(class, "close", "(Ljava/lang/String;)V", &[Arg::Text(id)]);
    }
}

fn usb_ports() -> Vec<crate::boards::PortInfo> {
    let (Some(vm), Some(class)) = (VM.get(), USB_SERIAL.get()) else { return Vec::new() };
    string_array(vm, class, "availableDevicesInfo").iter().filter_map(|line| crate::platformserial::port_from_info(line)).collect()
}

#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *mut c_void) -> jint {
    let Ok(mut env) = vm.get_env() else { return JNI_VERSION_1_6 };
    if let Some(bridge) = env.find_class(BRIDGE_CLASS).ok().and_then(|class| env.new_global_ref(class).ok()) {
        let _ = BRIDGE.set(bridge);
    }
    if let Some(serial) = env.find_class(USB_SERIAL_CLASS).ok().and_then(|class| env.new_global_ref(class).ok()) {
        let _ = USB_SERIAL.set(serial);
        crate::platformserial::install(crate::platformserial::Hooks { open: usb_open, write: usb_write, close: usb_close, ports: usb_ports });
    }
    if let Some(bluetooth) = env.find_class(BLUETOOTH_CLASS).ok().and_then(|class| env.new_global_ref(class).ok()) {
        let _ = BLUETOOTH.set(bluetooth);
        crate::platformbluetooth::install(crate::platformbluetooth::Hooks { open: bluetooth_open, write: bluetooth_write, close: bluetooth_close, devices: bluetooth_devices, scan: bluetooth_scan, scanning: bluetooth_scanning });
    }
    if let Some(wfb) = env.find_class(WFB_USB_CLASS).ok().and_then(|class| env.new_global_ref(class).ok()) {
        let _ = WFB_USB.set(wfb);
        if let Some(library) = wfb_library() {
            crate::wfbhost::register_usb(crate::wfbhost::UsbRadio {
                library,
                usb: crate::wfbhost::UsbHooks { devices: wfb_devices, open: wfb_open, close: wfb_close },
                opened: Mutex::new(None),
            });
        }
    }
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    if let Some(disks) = env.find_class(USB_DISKS_CLASS).ok().and_then(|class| env.new_global_ref(class).ok()) {
        let _ = USB_DISKS.set(disks);
        crate::platformdisk::install(crate::platformdisk::Hooks { disks: disk_list, open: disk_open, close: disk_close });
    }
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    let _ = VM.set(vm);
    if let Some(vm) = VM.get() {
        crate::androidvideo::start(vm);
    }
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
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_mapTile(mut env: JNIEnv, _class: JClass, map_type: JString, x: jint, y: jint, zoom: jint, cache_file: JString) -> jbyteArray {
    let provider = text_of(&mut env, &map_type).to_string_lossy().into_owned();
    let cache_path = text_of(&mut env, &cache_file).to_string_lossy().into_owned();
    let cache = (!cache_path.is_empty()).then(|| crate::tilecache::Cache::open(std::path::Path::new(&cache_path)).ok()).flatten();
    let persist = !crate::settingsstore::raw_setting("settings.appSettings.disableAllPersistence").and_then(|v| v.as_bool()).unwrap_or(false);
    let image = crate::maptiles::fetch_remembered(&provider, x, y, zoom, cache.as_ref(), persist);
    image.and_then(|bytes| env.byte_array_from_slice(&bytes).ok()).map_or(std::ptr::null_mut(), JByteArray::into_raw)
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
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCBridge_videoSetSurface(env: JNIEnv, _class: JClass, channel: jint, surface: JObject) -> jboolean {
    let Some(video) = crate::androidvideo::video() else { return 0 };
    jboolean::from(unsafe { (video.set_surface)(env.get_raw(), channel, surface.as_raw()) })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCUsbSerialManager_nativeDeviceNewData(env: JNIEnv, _class: JClass, pointer: jlong, data: JByteArray) {
    if let Ok(bytes) = env.convert_byte_array(&data) {
        crate::platformserial::received(pointer as u32, bytes);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCUsbSerialManager_nativeDeviceHasDisconnected(_env: JNIEnv, _class: JClass, pointer: jlong) {
    USB_DEVICES.lock().unwrap_or_else(PoisonError::into_inner).remove(&(pointer as u32));
    crate::platformserial::closed(pointer as u32, "The USB device was disconnected.");
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_mavlink_qgroundcontrol_QGCUsbSerialManager_nativeDeviceException(mut env: JNIEnv, _class: JClass, pointer: jlong, message: JString) {
    let message = text_of(&mut env, &message).to_string_lossy().into_owned();
    crate::platformserial::closed(pointer as u32, &message);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_one_aircast_android_BluetoothLinks_nativeNewData(env: JNIEnv, _class: JClass, pointer: jlong, data: JByteArray) {
    if let Ok(bytes) = env.convert_byte_array(&data) {
        crate::platformbluetooth::received(pointer as u32, bytes);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_one_aircast_android_BluetoothLinks_nativeClosed(mut env: JNIEnv, _class: JClass, pointer: jlong, message: JString) {
    let message = text_of(&mut env, &message).to_string_lossy().into_owned();
    crate::platformbluetooth::closed(pointer as u32, &message);
}

fn reading(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_one_aircast_android_GcsLocation_nativeSource(mut env: JNIEnv, _class: JClass, token: JString) {
    let token = text_of(&mut env, &token).to_string_lossy().into_owned();
    crate::gcsposition::lock().host_source(crate::gcsposition::Source::from_token(&token));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_one_aircast_android_GcsLocation_nativeError(_env: JNIEnv, _class: JClass, code: jint) {
    crate::gcsposition::report_error(i64::from(code));
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_one_aircast_android_GcsLocation_nativeUpdate(
    _env: JNIEnv,
    _class: JClass,
    latitude: f64,
    longitude: f64,
    altitude: f64,
    horizontal_accuracy_m: f64,
    vertical_accuracy_m: f64,
    direction_deg: f64,
    direction_accuracy_deg: f64,
    ground_speed_m_s: f64,
) {
    crate::gcsposition::report(crate::gcsposition::Update {
        latitude: reading(latitude),
        longitude: reading(longitude),
        altitude: reading(altitude),
        horizontal_accuracy_m: reading(horizontal_accuracy_m),
        vertical_accuracy_m: reading(vertical_accuracy_m),
        direction_deg: reading(direction_deg),
        direction_accuracy_deg: reading(direction_accuracy_deg),
        ground_speed_m_s: reading(ground_speed_m_s),
    });
}
