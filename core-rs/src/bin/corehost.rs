use std::ffi::{CString, c_char, c_int};

use qgc_core::nativehost::{qgc_run, qgc_shutdown, qgc_start};

fn main() {
    let given: Vec<String> = std::env::args().collect();
    let autoconnect = given.iter().any(|a| a == "--autoconnect");
    let arguments: Vec<CString> = given.into_iter().filter(|a| a != "--autoconnect").chain((!autoconnect).then(|| "--no-autoconnect".to_string())).filter_map(|a| CString::new(a).ok()).collect();
    let pointers: Vec<*const c_char> = arguments.iter().map(|a| a.as_ptr()).collect();
    let started = unsafe { qgc_start(c_int::try_from(pointers.len()).unwrap_or(0), pointers.as_ptr()) };
    if started != 0 {
        std::process::exit(started);
    }
    let code = qgc_run();
    qgc_shutdown();
    std::process::exit(code);
}
