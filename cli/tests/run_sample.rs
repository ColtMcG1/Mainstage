use std::process::Command;
use std::path::PathBuf;

#[test]
fn run_cli_sample_should_print_file_contents_not_null() {
    // Locate compiled binary in target/debug
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let mut bin = PathBuf::from(&manifest_dir);
    bin.push("target");
    bin.push("debug");
    bin.push(if cfg!(windows) { "mainstage.exe" } else { "mainstage" });

    // sample script path relative to CLI crate
    let mut sample = PathBuf::from(&manifest_dir);
    sample.push("samples");
    sample.push("e2e");
    sample.push("2.ms");

    let output = Command::new(bin.as_os_str())
        .arg("run")
        .arg(sample.as_os_str())
        .output()
        .expect("failed to spawn mainstage binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // Expect output to include contents of sample file, not bare "Null"
    assert!(!stdout.trim().ends_with("Null"), "Runtime printed Null unexpectedly: {}", stdout);
}
