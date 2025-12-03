use std::process::Command;
use std::path::Path;

#[test]
fn list_compilers_smoke() {
    // Locate the workspace root from the crate manifest dir, then target/debug
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // crate: plugin/cpp_plugin -> workspace root is two levels up
    let workspace = manifest.parent().and_then(|p| p.parent()).expect("expected workspace parent");
    // Try several likely locations for the built binary: workspace target, crate target, plugin dir target
    let mut candidates = Vec::new();
    if cfg!(windows) {
        candidates.push(workspace.join("target").join("debug").join("cpp_plugin.exe"));
        candidates.push(manifest.join("target").join("debug").join("cpp_plugin.exe"));
        if let Some(parent) = manifest.parent() { candidates.push(parent.join("target").join("debug").join("cpp_plugin.exe")); }
    } else {
        candidates.push(workspace.join("target").join("debug").join("cpp_plugin"));
        candidates.push(manifest.join("target").join("debug").join("cpp_plugin"));
        if let Some(parent) = manifest.parent() { candidates.push(parent.join("target").join("debug").join("cpp_plugin")); }
    }

    let exe = candidates.into_iter().find(|p| p.exists()).expect("plugin binary not found in candidate locations; run `cargo build` first");

    let output = Command::new(&exe)
        .arg("call")
        .arg("list_compilers")
        .output()
        .expect("failed to run plugin binary");

    assert!(output.status.success(), "plugin exited with non-zero status");
    let out = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&out).expect("output was not valid JSON");
    assert!(v.is_array(), "expected JSON array, got: {}", out);
}
