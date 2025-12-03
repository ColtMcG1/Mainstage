use std::process::Command;
use std::path::Path;
use std::fs;

#[test]
fn compile_smoke() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest.parent().and_then(|p| p.parent()).expect("expected workspace parent");

    // locate plugin exe
    let mut candidates = Vec::new();
    if cfg!(windows) {
        candidates.push(workspace.join("target").join("debug").join("c_plugin.exe"));
        candidates.push(manifest.join("target").join("debug").join("c_plugin.exe"));
        if let Some(parent) = manifest.parent() { candidates.push(parent.join("target").join("debug").join("c_plugin.exe")); }
    } else {
        candidates.push(workspace.join("target").join("debug").join("c_plugin"));
        candidates.push(manifest.join("target").join("debug").join("c_plugin"));
        if let Some(parent) = manifest.parent() { candidates.push(parent.join("target").join("debug").join("c_plugin")); }
    }

    let exe = match candidates.into_iter().find(|p| p.exists()) {
        Some(p) => p,
        None => panic!("plugin binary not found in candidate locations; run `cargo build` first"),
    };

    // create temp dir and tiny C source
    let td = tempfile::tempdir().expect("tempdir");
    let dir = td.path();
    let src = r#"int main(){ return 0; }"#;
    let src_path = dir.join("hello.c");
    fs::write(&src_path, src).expect("write source");

    // query compilers first via plugin; if none, skip test
    let list_out = Command::new(&exe).arg("call").arg("list_compilers").output().expect("list_compilers failed");
    if !list_out.status.success() {
        panic!("list_compilers failed to run");
    }
    let list_json: serde_json::Value = serde_json::from_slice(&list_out.stdout).unwrap_or(serde_json::json!([]));
    if !list_json.is_array() || list_json.as_array().unwrap().is_empty() {
        eprintln!("No compilers found on host; skipping compile_smoke test");
        return;
    }

    // prepare compile request JSON (args array: [ [sources], [flags] ])
    let req = serde_json::json!({ "args": [ [ src_path.to_string_lossy() ], [] ] });
    let req_s = req.to_string();

    let mut cmd = Command::new(&exe);
    cmd.arg("call").arg("compile");
    cmd.current_dir(dir);

    let output = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(req_s.as_bytes())?;
            child.wait_with_output()
        })
        .expect("failed to run compile");

    assert!(output.status.success(), "compile process failed: {}", String::from_utf8_lossy(&output.stderr));
    let out_json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("invalid json");
    assert!(out_json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false), "compile reported failure: {}", out_json);

    // Confirm output binary exists (path may be relative)
    if let Some(path_val) = out_json.get("path").and_then(|v| v.as_str()) {
        let out_path = Path::new(path_val);
        let resolved = if out_path.is_absolute() { out_path.to_path_buf() } else { dir.join(out_path) };
        assert!(resolved.exists(), "compiled binary not found at {:?}", resolved);
    } else {
        panic!("compile result missing path: {}", out_json);
    }
}
