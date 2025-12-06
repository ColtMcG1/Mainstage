use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// Minimal in-process adapter for asm_plugin. Exposes `plugin_name`,
// `plugin_call_json` and `plugin_free` and delegates to the same logic
// used by the CLI binary (assembly functions) via the `common` crate when possible.

fn list_compilers_json() -> String {
    let found = common::find_available_compilers_from(&["nasm", "yasm", "gcc", "clang"]);
    let mut out: Vec<serde_json::Value> = Vec::new();
    for (name, path) in found.into_iter() {
        let version = common::get_compiler_version(path.as_path()).unwrap_or_default();
        out.push(serde_json::json!({ "name": name, "path": path.to_string_lossy(), "version": version }));
    }
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".to_string())
}

fn assemble_json(args_json: &serde_json::Value) -> String {
    let mut sources: Vec<String> = Vec::new();
    let mut flags: Vec<String> = Vec::new();
    let mut compiler: Option<String> = None;

    match args_json {
        serde_json::Value::Array(a) => {
            if let Some(sv) = a.get(0) { if let serde_json::Value::Array(sa) = sv { sources = sa.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(); } }
            if let Some(fv) = a.get(1) { if let serde_json::Value::Array(fa) = fv { flags = fa.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(); } }
            if let Some(cv) = a.get(2) { if let Some(s) = cv.as_str() { compiler = Some(s.to_string()); } }
        }
        serde_json::Value::Object(map) => {
            if let Some(sv) = map.get("sources") { if let serde_json::Value::Array(sa) = sv { sources = sa.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(); } }
            if let Some(fv) = map.get("flags") { if let serde_json::Value::Array(fa) = fv { flags = fa.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(); } }
            if let Some(cv) = map.get("compiler") { if let Some(s) = cv.as_str() { compiler = Some(s.to_string()); } }
        }
        _ => {}
    }

    // Inline assemble implementation using helpers from `common`.
    fn candidate_compilers() -> Vec<&'static str> {
        #[cfg(target_os = "windows")] return vec!["ml64", "ml", "nasm", "yasm", "cl", "gcc", "clang"];
        #[cfg(not(target_os = "windows"))] return vec!["nasm", "yasm", "gcc", "clang"];
    }

    fn find_available_compilers() -> Vec<(String, std::path::PathBuf)> {
        common::find_available_compilers_from(&candidate_compilers())
    }

    fn select_compiler(hint: Option<&str>) -> Option<(String, std::path::PathBuf)> {
        if let Some(h) = hint {
            if let Ok(p) = which::which(h) {
                return Some((h.to_string(), p));
            }
        }
        find_available_compilers().into_iter().next()
    }

    fn assemble_sources_with(sources: &[String], flags: &[String], compiler_hint: Option<&str>) -> Result<String, String> {
        if sources.is_empty() {
            return Err("No source files provided".to_string());
        }

        let (compiler_name, compiler_path) = match select_compiler(compiler_hint) {
            Some(p) => p,
            None => return Err("No supported assembler/compiler found on the system".to_string()),
        };

        let out_name = if cfg!(target_os = "windows") { "output_binary.exe" } else { "output_binary" };

        let mut cmd = common::build_compile_command(&compiler_name, &compiler_path, sources, flags, out_name);

        if cfg!(target_os = "windows") {
            if let Some(envs) = common::ensure_msvc_env(compiler_path.as_path()) {
                cmd.envs(envs.into_iter());
            }
        }

        let output = cmd.output().map_err(|e| format!("Failed to execute assembler/compiler '{}': {}", compiler_name, e))?;
        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Assembly failed: stdout:\n{}\nstderr:\n{}", stdout, stderr));
        }

        Ok(out_name.to_string())
    }

    let result = assemble_sources_with(&sources, &flags, compiler.as_deref());
    let output = match result {
        Ok(path) => serde_json::json!({"ok": true, "path": path}),
        Err(err) => serde_json::json!({"ok": false, "error": err}),
    };
    serde_json::to_string(&output).unwrap_or("null".to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_name() -> *const c_char {
    let s = CString::new("asm_plugin").unwrap();
    s.into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_call_json(func: *const c_char, args_json: *const c_char) -> *mut c_char {
    unsafe {
        let func = if func.is_null() { "" } else { CStr::from_ptr(func).to_str().unwrap_or("") };
        let args = if args_json.is_null() { serde_json::json!(null) } else {
            match CStr::from_ptr(args_json).to_str() {
                Ok(s) => serde_json::from_str(s).unwrap_or(serde_json::json!(null)),
                Err(_) => serde_json::json!(null),
            }
        };
        let res = match func {
            "list_compilers" => list_compilers_json(),
            "compile" | "assemble" => assemble_json(&args),
            _ => serde_json::json!({"error": "unknown function"}).to_string(),
        };
        CString::new(res).unwrap().into_raw()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_free(ptr: *mut c_char) {
    if ptr.is_null() { return }
    unsafe { let _ = CString::from_raw(ptr); }; // reclaim
}
