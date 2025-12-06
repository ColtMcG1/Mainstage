//! file: core/src/vm/inprocess.rs
//! description: in-process dynamic library plugin adapter using `libloading`.
//!
//! This adapter loads a shared library at runtime and resolves the minimal
//! C-style symbols (`plugin_name`, `plugin_call_json`, `plugin_free`) so the
//! host can call plugin functions without spawning a separate process.

use std::ffi::{CStr, CString};
use std::io::Seek;
use std::os::raw::c_char;
use std::path::Path;
use std::sync::Arc;

use crate::vm::plugin::{Plugin, PluginMetadata};
use crate::vm::value::{Value as VmValue, json_to_value, values_to_json_array};
use async_trait::async_trait;
use libloading::{Library, Symbol};
use serde_json::Value as JsonValue;

type PluginNameFn = unsafe extern "C" fn() -> *const c_char;
type PluginCallJsonFn =
    unsafe extern "C" fn(func: *const c_char, args_json: *const c_char) -> *mut c_char;
type PluginFreeFn = unsafe extern "C" fn(ptr: *mut c_char);

pub struct InProcessPlugin {
    _lib: Arc<Library>,
    name: String,
    _name_fn: PluginNameFn,
    call_fn: PluginCallJsonFn,
    free_fn: Option<PluginFreeFn>,
}

impl InProcessPlugin {
    pub fn new(path: &Path) -> Result<Self, String> {
        // Validate path exists and is a file before attempting to load.
        if !path.exists() {
            return Err(format!("library path does not exist: {}", path.display()));
        }
        match std::fs::metadata(path) {
            Ok(m) if m.is_file() => {}
            _ => return Err(format!("library path is not a file: {}", path.display())),
        }

        unsafe {
            let lib = Library::new(path).map_err(|e| {
                // Best-effort: try to detect binary format and suggest arch mismatches
                let mut hint = String::new();
                if let Ok(ba) = guess_binary_arch(path) {
                    let host = std::env::consts::ARCH.to_string();
                    if ba != host {
                        hint = format!(" Detected binary arch '{}', host arch '{}'.", ba, host);
                    }
                }
                format!(
                    "failed to load library {}: {}. Hint: verify the file is a valid shared library for this OS/architecture and that it exports the expected symbols.{}",
                    path.display(), e, hint
                )
            })?;

            // Resolve plugin_name symbol
            let name_sym: Symbol<PluginNameFn> = lib.get(b"plugin_name\0").map_err(|e| {
                format!(
                    "missing symbol 'plugin_name' in {}: {}. Ensure the plugin exports the C symbol 'plugin_name' with 'extern \"C\"' and `#[no_mangle]`.",
                    path.display(), e
                )
            })?;
            let name_fn = *name_sym;
            let raw = name_fn();
            if raw.is_null() {
                return Err(format!(
                    "plugin_name returned null for library {}",
                    path.display()
                ));
            }
            let cname = match CStr::from_ptr(raw).to_str() {
                Ok(s) if !s.trim().is_empty() => s.to_string(),
                Ok(_) => {
                    return Err(format!(
                        "plugin_name returned empty string in {}",
                        path.display()
                    ));
                }
                Err(e) => {
                    return Err(format!(
                        "plugin_name returned invalid UTF-8 in {}: {}",
                        path.display(),
                        e
                    ));
                }
            };

            // Resolve call symbol
            let call_sym: Symbol<PluginCallJsonFn> = lib.get(b"plugin_call_json\0").map_err(|e| {
                format!(
                    "missing symbol 'plugin_call_json' in {}: {}. Ensure the plugin exports 'plugin_call_json' and follows the ABI (func, args_json) -> char*.",
                    path.display(), e
                )
            })?;
            let call_fn = *call_sym;
            /// Try to guess the binary architecture from file headers (PE/ELF/Mach-O).
            fn guess_binary_arch(path: &Path) -> Result<String, String> {
                use std::io::Read;
                let mut f = std::fs::File::open(path).map_err(|e| format!("open failed: {}", e))?;
                let mut buf = [0u8; 64];
                let n = f
                    .read(&mut buf)
                    .map_err(|e| format!("read failed: {}", e))?;
                if n >= 4 && &buf[0..4] == b"\x7fELF" {
                    // ELF: e_ident[4] is class: 1=32,2=64. e_machine at offset 18 (little-endian)
                    if n > 18 {
                        let class = buf[4];
                        let emachine = u16::from_le_bytes([buf[18], buf[19]]);
                        match (class, emachine) {
                            (2, 62) => return Ok("x86_64".to_string()),
                            (1, 3) => return Ok("x86".to_string()),
                            (_, 183) => return Ok("aarch64".to_string()),
                            _ => return Ok(format!("elf-emu-{}", emachine)),
                        }
                    }
                }
                if n >= 2 && &buf[0..2] == b"MZ" {
                    // PE header: at offset 0x3c is e_lfanew (u32 LE)
                    let mut f2 =
                        std::fs::File::open(path).map_err(|e| format!("open2 failed: {}", e))?;
                    let mut hdr = [0u8; 64];
                    f2.read_exact(&mut hdr).ok();
                    let e_lfanew =
                        u32::from_le_bytes([hdr[0x3c], hdr[0x3d], hdr[0x3e], hdr[0x3f]]) as usize;
                    let mut pehdr = vec![0u8; 8];
                    f2.seek(std::io::SeekFrom::Start(e_lfanew as u64)).ok();
                    f2.read_exact(&mut pehdr).ok();
                    // machine is at offset e_lfanew + 4 (IMAGE_FILE_HEADER.Machine is u16)
                    let mut mh = [0u8; 2];
                    f2.read_exact(&mut mh).ok();
                    let machine = u16::from_le_bytes(mh);
                    match machine {
                        0x8664 => return Ok("x86_64".to_string()),
                        0x014c => return Ok("x86".to_string()),
                        0xAA64 => return Ok("aarch64".to_string()),
                        _ => return Ok(format!("pe-0x{:x}", machine)),
                    }
                }
                if n >= 4 {
                    let magic = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                    // Mach-O / Fat headers: 0xFEEDFACE, 0xFEEDFACF, 0xCEFAEDFE, 0xCFFAEDFE, 0xCAFEBABE
                    match magic {
                        0xFEEDFACF | 0xFEEDFACE | 0xCAFEBABE => {
                            // best effort: assume 64-bit for FEEDFACF
                            if magic == 0xFEEDFACF {
                                return Ok("x86_64".to_string());
                            } else {
                                return Ok("unknown-mach-o".to_string());
                            }
                        }
                        _ => {}
                    }
                }
                Err("unknown binary format".to_string())
            }

            // Optional free symbol
            let free_fn = match lib.get::<PluginFreeFn>(b"plugin_free\0") {
                Ok(s) => Some(*s),
                Err(_) => None,
            };

            Ok(InProcessPlugin {
                _lib: Arc::new(lib),
                name: cname,
                _name_fn: name_fn,
                call_fn,
                free_fn,
            })
        }
    }
}

#[async_trait]
impl Plugin for InProcessPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn call(&self, func: &str, args: Vec<VmValue>) -> Result<VmValue, String> {
        // Serialize args to JSON array
        let j = values_to_json_array(&args);
        let args_json = serde_json::to_string(&j).map_err(|e| format!("serialize args: {}", e))?;
        let cfunc = CString::new(func).map_err(|e| format!("func name: {}", e))?;
        let cargs = CString::new(args_json).map_err(|e| format!("args json: {}", e))?;

        unsafe {
            let out_ptr = (self.call_fn)(cfunc.as_ptr(), cargs.as_ptr());
            if out_ptr.is_null() {
                return Err("plugin returned null".into());
            }
            let out_cstr = CStr::from_ptr(out_ptr);
            let out_str = out_cstr.to_string_lossy().into_owned();

            // free memory
            if let Some(free_sym) = &self.free_fn {
                free_sym(out_ptr);
            } else {
                libc::free(out_ptr as *mut libc::c_void);
            }

            let json_val: JsonValue = serde_json::from_str(&out_str)
                .map_err(|e| format!("invalid json from plugin: {}", e))?;
            let vm_val = json_to_value(&json_val);
            Ok(vm_val)
        }
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::default()
    }
}
