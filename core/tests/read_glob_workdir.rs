use std::fs;

// These are integration-style tests that invoke the VM runner on a small
// script located in a temporary directory and assert that host functions
// `read` and globbing resolve paths relative to the script file's directory.

#[test]
fn test_read_resolves_relative_to_script_dir() {
    // create a temp dir and file
    let td = tempfile::tempdir().expect("tempdir");
    let dir = td.path();
    let file_path = dir.join("data.txt");
    fs::write(&file_path, "hello-world\n").expect("write file");

    // script that calls read("./data.txt") via the `read` host function
    let script_src = r#"
[entrypoint]
workspace w {
    projects = [];
}

stage L1(var) {
    in = read(var);
    say(in[0]);
}

stage main() {
    sources = ["data.txt"];
    L1(sources[0]);
}
"#;

    let script_file = dir.join("script.ms");
    fs::write(&script_file, script_src).expect("write script");

    // Run the core pipeline directly in-process: parse, analyze, lower, emit
    // bytecode and run the VM. This avoids invoking the CLI binary and lets
    // us capture VM debug output directly.
    let script = mainstage_core::script::Script::new(script_file.clone()).expect("Failed to load script file");
    let mut ast = mainstage_core::ast::generate_ast_from_source(&script).expect("generate ast");
    let (entry, analysis) = match mainstage_core::analyzers::semantic::analyze_semantic_rules(&mut ast, None) {
        Ok((e,a)) => (e,a),
        Err(diags) => panic!("analysis diags: {:?}", diags),
    };
    mainstage_core::analyzers::acyclic::analyze_acyclic_rules(&ast).expect("acyclic");
    let ir_module = mainstage_core::ir::lower_ast_to_ir(&ast, &entry, false, Some(&analysis));
    let bytecode = mainstage_core::ir::emit_bytecode(&ir_module);

    // Run VM with trace enabled (this prints to stderr/stdout from VM)
    let result = mainstage_core::VM::new(bytecode).run(true);
    assert!(result.is_ok(), "VM run failed: {:?}", result.err());
}

#[test]
fn test_glob_resolves_relative_to_script_dir() {
    let td = tempfile::tempdir().expect("tempdir");
    let dir = td.path();
    let file1 = dir.join("a.ms");
    let file2 = dir.join("b.ms");
    fs::write(&file1, "content-a").expect("write a");
    fs::write(&file2, "content-b").expect("write b");

    let script_src = r#"
[entrypoint]
workspace w {
    projects = [];
}

stage L1(var) {
    in = read(var);
    say(in[0]);
}

stage main() {
    sources = ["*.ms"];
    L1(sources[0]);
}
"#;
    let script_file = dir.join("script.ms");
    fs::write(&script_file, script_src).expect("write script");

     // Run the core pipeline directly in-process: parse, analyze, lower, emit
     // bytecode and run the VM. This avoids invoking the CLI binary and lets
     // us capture VM debug output directly.
     let script = mainstage_core::script::Script::new(script_file.clone()).expect("Failed to load script file");
     let mut ast = mainstage_core::ast::generate_ast_from_source(&script).expect("generate ast");
     let (entry, analysis) = match mainstage_core::analyzers::semantic::analyze_semantic_rules(&mut ast, None) {
          Ok((e,a)) => (e,a),
          Err(diags) => panic!("analysis diags: {:?}", diags),
     };
     mainstage_core::analyzers::acyclic::analyze_acyclic_rules(&ast).expect("acyclic");
     let ir_module = mainstage_core::ir::lower_ast_to_ir(&ast, &entry, false, Some(&analysis));
     let bytecode = mainstage_core::ir::emit_bytecode(&ir_module);

     // Run VM with trace enabled (this prints to stderr/stdout from VM)
     let result = mainstage_core::VM::new(bytecode).run(true);
     assert!(result.is_ok(), "VM run failed: {:?}", result.err());
}
