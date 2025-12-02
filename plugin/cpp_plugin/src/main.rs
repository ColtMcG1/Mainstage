use std::io::{self, Read};

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_default();
    let func = args.next().unwrap_or_default();

    if cmd != "call" {
        eprintln!("unsupported command");
        std::process::exit(1);
    }

    // Read JSON request from stdin (we'll accept either full object or just args array)
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).unwrap_or(0);
    let json: serde_json::Value = if buf.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&buf).unwrap_or(serde_json::json!({}))
    };

    // Provide a `list_compilers` function to enumerate available compilers.
    if func == "list_compilers" {
        #[cfg(target_os = "windows")]
        let candidates = vec!["g++", "clang++", "cl"];
        #[cfg(not(target_os = "windows"))]
        let candidates = vec!["g++", "clang++", "clang", "gcc"];

        let mut found: Vec<serde_json::Value> = Vec::new();
        for c in candidates.iter() {
            if let Ok(path) = which::which(c) {
                found.push(serde_json::json!({ "name": c, "path": path.to_string_lossy() }));
            }
        }
        let s = serde_json::to_string(&found).unwrap_or("[]".to_string());
        println!("{}", s);
        return;
    }

    // For `compile`, accept either an args-array or args-object.
    if func == "compile" {
        // defaults
        let mut sources: Vec<String> = Vec::new();
        let mut flags: Vec<String> = Vec::new();
        let mut compiler: Option<String> = None;

        match &json["args"] {
            serde_json::Value::Array(a) => {
                // args array: [sources, flags?, compiler?]
                if let Some(sv) = a.get(0) {
                    if let serde_json::Value::Array(sa) = sv {
                        sources = sa.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                    }
                }
                if let Some(fv) = a.get(1) {
                    if let serde_json::Value::Array(fa) = fv {
                        flags = fa.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                    }
                }
                if let Some(cv) = a.get(2) {
                    if let Some(s) = cv.as_str() { compiler = Some(s.to_string()); }
                }
            }
            serde_json::Value::Object(map) => {
                if let Some(sv) = map.get("sources") {
                    if let serde_json::Value::Array(sa) = sv { sources = sa.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(); }
                }
                if let Some(fv) = map.get("flags") {
                    if let serde_json::Value::Array(fa) = fv { flags = fa.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(); }
                }
                if let Some(cv) = map.get("compiler") { if let Some(s) = cv.as_str() { compiler = Some(s.to_string()); } }
            }
            _ => {}
        }

        let result = compile_cpp_sources_with(sources, flags, compiler);
        let output = match result {
            Ok(path) => serde_json::json!({"ok": true, "path": path}),
            Err(err) => serde_json::json!({"ok": false, "error": err}),
        };
        let s = serde_json::to_string(&output).unwrap_or("null".to_string());
        println!("{}", s);
        return;
    }

    eprintln!("unknown function");
    std::process::exit(1);
}

/// Compiles the given C++ source files and returns the path to the compiled binary or an error message.
/// # Arguments
/// * `sources` - A vector of strings representing the paths to C++ source files.
/// # Returns
/// * `Ok(String)` - The path to the compiled binary if compilation is successful.
/// * `Err(String)` - An error message if compilation fails.
fn compile_cpp_sources_with(sources: Vec<String>, flags: Vec<String>, compiler_hint: Option<String>) -> Result<String, String> {
    if sources.is_empty() {
        return Err("No source files provided".to_string());
    }
    // Determine candidate compilers and prefer `compiler_hint` if provided.
    #[cfg(target_os = "windows")]
    let system_support_compilers = vec!["g++", "clang++", "cl"];
    #[cfg(not(target_os = "windows"))]
    let system_support_compilers = vec!["g++", "clang++", "clang", "gcc"];

    // If the caller specified a compiler, try that first
    let mut candidates: Vec<String> = Vec::new();
    if let Some(ch) = compiler_hint {
        candidates.push(ch);
    }
    for c in system_support_compilers.iter() {
        if !candidates.contains(&c.to_string()) {
            candidates.push(c.to_string());
        }
    }

    // Find the first available compiler executable
    let chosen = candidates.into_iter().find(|c| which::which(c).is_ok());
    let compiler = match chosen {
        Some(c) => c,
        None => return Err("No supported C++ compiler found on the system".to_string()),
    };

    // Prepare output binary name
    let out_name = if cfg!(target_os = "windows") { "output_binary.exe" } else { "output_binary" };

    // Build command
    let mut command = std::process::Command::new(&compiler);
    // For MSVC (cl), arguments differ: cl <src> /Fe:out.exe <flags>
    if compiler == "cl" || compiler.to_lowercase().contains("cl") {
        for src in &sources { command.arg(src); }
        for f in flags.iter() { command.arg(f); }
        command.arg(format!("/Fe:{}", out_name));
    } else {
        for src in &sources { command.arg(src); }
        for f in flags.iter() { command.arg(f); }
        command.arg("-o");
        command.arg(out_name);
    }

    // Execute
    let output = command.output().map_err(|e| format!("Failed to execute compiler '{}': {}", compiler, e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Compilation failed: {}", stderr));
    }
    Ok(out_name.to_string())
}