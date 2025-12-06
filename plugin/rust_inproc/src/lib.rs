use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// Minimal example plugin: responds to any call with a JSON result.
// Exports:
// - plugin_name() -> const char*
// - plugin_call_json(func, args_json) -> char* (malloc'd string)
// - plugin_free(ptr)

#[no_mangle]
pub extern "C" fn plugin_name() -> *const c_char {
    static NAME: &str = "rust_inproc";
    NAME.as_ptr() as *const _ as *const c_char
}

#[no_mangle]
pub extern "C" fn plugin_call_json(func: *const c_char, args_json: *const c_char) -> *mut c_char {
    unsafe {
        let f = if func.is_null() { "" } else { CStr::from_ptr(func).to_str().unwrap_or("") };
        let args = if args_json.is_null() { "null" } else { CStr::from_ptr(args_json).to_str().unwrap_or("null") };
        // Very small example: echo back func and args
        let resp = format!("{{\"ok\": true, \"func\": \"{}\", \"args\": {} }}", f, args);
        let c = CString::new(resp).unwrap();
        c.into_raw()
    }
}

#[no_mangle]
pub extern "C" fn plugin_free(ptr: *mut c_char) {
    if ptr.is_null() { return; }
    unsafe { let _ = CString::from_raw(ptr); }
}
