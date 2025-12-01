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

    // For `echo`, return first arg as-is (or empty string)
    if func == "compile" {
        let out = match &json["args"] {
            serde_json::Value::Array(a) => a.get(0).cloned().unwrap_or(serde_json::Value::String("Unable to read input".to_string())),
            _ => serde_json::Value::String("Unknown input".to_string()),
        };
        let sources: Vec<String> = match out {
            serde_json::Value::Array(arr) => arr.into_iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
            _ => vec![],
        };
        let result = compile_cpp_sources(sources);
        let output = match result {
            Ok(path) => serde_json::json!(path),
            Err(err) => serde_json::json!(err),
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
fn compile_cpp_sources(sources: Vec<String>) -> Result<String, String> {
    if sources.is_empty() {
        return Err("No source files provided".to_string());
    }

    #[cfg(target_os = "windows")]
    let system_support_compilers = vec!["g++", "clang++", "cl", "msvc"];
    #[cfg(not(target_os = "windows"))]
    let system_support_compilers = vec!["g++", "clang++", "clang", "gcc"];

    let mut available_compilers = vec![];
    for compiler in system_support_compilers.iter() {
        if which::which(compiler).is_ok() {
            available_compilers.push(compiler.to_string());
        }
    }

    if available_compilers.is_empty() {
        return Err("No supported C++ compiler found on the system".to_string());
    }

    match available_compilers[0].as_str() {
        "g++" | "clang++" | "clang" | "gcc" => {
            let mut command = std::process::Command::new(&available_compilers[0]);
            for src in sources.iter() {
                command.arg(src);
            }
            command.arg("-o").arg("output_binary");
            let output = command.output().map_err(|e| format!("Failed to execute compiler: {}", e))?;
            if !output.status.success() {
                return Err(format!("Compilation failed: {}", String::from_utf8_lossy(&output.stderr)));
            }
            Ok("output_binary".to_string())
        }
        "cl" | "msvc" => {
            let mut command = std::process::Command::new(&available_compilers[0]);
            for src in sources.iter() {
                command.arg(src);
            }
            command.arg("/Fe:output_binary.exe");
            let output = command.output().map_err(|e| format!("Failed to execute compiler: {}", e))?;
            if !output.status.success() {
                return Err(format!("Compilation failed: {}", String::from_utf8_lossy(&output.stderr)));
            }
            Ok("output_binary.exe".to_string())
        }
        _ => return Err("Unsupported compiler".to_string()),
    }
}