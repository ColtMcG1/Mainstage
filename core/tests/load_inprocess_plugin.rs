// This test is ignored by default; it exercises loading the example
// `plugin/rust_inproc` cdylib. Build the plugin first with:
//   cd plugin/rust_inproc; cargo build --release
// Then run this test manually with: `cargo test --test load_inprocess_plugin -- --ignored`

use mainstage_core::VM;

#[test]
#[ignore]
fn load_example_inprocess_plugin() {
    // Assume repo root layout: plugin/rust_inproc contains manifest + built library
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..\\..\\plugin\\rust_inproc");
    let scripts_dir = repo.canonicalize().expect("canonicalize plugin dir");

    let mut vm = VM::new(vec![]);
    // discover_plugins expects plugin directory parent; point to repo plugin directory
    let res = vm.discover_plugins(Some(scripts_dir));
    assert!(res.is_ok(), "discover failed: {:?}", res.err());
    // Ensure the example plugin is registered
    let names = vm.registered_plugin_names();
    assert!(names.iter().any(|n| n == "rust_inproc"), "plugin not registered");
}
