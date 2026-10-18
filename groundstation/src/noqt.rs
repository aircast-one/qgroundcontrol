use std::ffi::{CString, c_char};

fn given(text: &str) -> *mut c_char {
    CString::new(text).unwrap_or_default().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_get(_path: *const c_char) -> *mut c_char {
    given(r#"{"kind":"null"}"#)
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_get_fields(_path: *const c_char, _fields: *const c_char) -> *mut c_char {
    given(r#"{"kind":"null"}"#)
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_set(_path: *const c_char, _value: *const c_char) -> *mut c_char {
    given(r#"{"ok":false,"reason":"no host"}"#)
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_invoke(_path: *const c_char, _args: *const c_char) -> *mut c_char {
    given(r#"{"ok":false,"reason":"no host"}"#)
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_watch(_paths: *const c_char) {}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_remember_setting(_key: *const c_char, _value: *const c_char) {}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_set_event_handler(
    _handler: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_qt_free(text: *mut c_char) {
    if !text.is_null() {
        drop(unsafe { CString::from_raw(text) });
    }
}
