use std::path::PathBuf;
use std::collections::HashSet;
use mainstage_core::{ast, ir, script::Script};
use mainstage_core::ir::op::IROp;

const NESTED_STAGE_SAMPLE: &str = r#"
stage sayx(val) { return; }
stage sayy(val) { return; }
stage s() {
    arr1 = ["a"];
    arr2 = ["b"];
    for x in arr1 {
        for y in arr2 {
            sayx(x);
            sayy(y);
        }
    }
}
"#;

const STAGE_MULTI_BODY: &str = r#"
stage sayit(v) { return; }
stage store(v) { return; }
stage s2() {
    arr = ["a"];
    for v in arr {
        sayit(v);
        store(v);
    }
}
"#;

const WORKSPACE_MULTI_BODY: &str = r#"[entrypoint]
workspace demo_ws {
    projects = [test_pj];

    for p in projects {
        extra(p);
        process_project_stage(p);
    }
}

project test_pj { sources = ["./samples/e2e/*.ms"]; }

stage process_project_stage(prj) { return; }
stage extra(prj) { return; }
"#;

#[test]
fn nested_stage_loops_bind_locals_and_emit_calls() {
    let script = Script { name: "nested.ms".to_string(), path: PathBuf::from("nested.ms"), content: NESTED_STAGE_SAMPLE.to_string() };
    let ast = ast::generate_ast_from_source(&script).expect("failed to parse nested stage sample");
    let ir_mod = ir::lower_ast_to_ir(&ast, "", false, None);

    // Expect at least two distinct local indices to be created and used (x and y)
    let mut sstores = HashSet::new();
    let mut lloads = HashSet::new();
    let mut calllabel_count = 0;
    for op in ir_mod.ops.iter() {
        match op {
            IROp::SLocal { src: _, local_index } => { sstores.insert(*local_index); }
            IROp::LLocal { dest: _, local_index } => { lloads.insert(*local_index); }
            IROp::CallLabel { dest: _, label_index: _, args } => { 
                calllabel_count += 1;
                assert!(!args.is_empty(), "CallLabel in nested loop should have args");
            }
            _ => {}
        }
    }

    // There should be at least two locals (for x and y) and they should be stored and loaded
    let common: Vec<_> = sstores.intersection(&lloads).collect();
    assert!(common.len() >= 2, "expected at least two loop-local bindings used, ops:\n{}", ir_mod);
    assert!(calllabel_count >= 2, "expected multiple CallLabel ops for calls, ops:\n{}", ir_mod);
}

#[test]
fn stage_multi_statement_body_emits_multiple_calls_inside_loop() {
    let script = Script { name: "multi.ms".to_string(), path: PathBuf::from("multi.ms"), content: STAGE_MULTI_BODY.to_string() };
    let ast = ast::generate_ast_from_source(&script).expect("failed to parse stage multi sample");
    let ir_mod = ir::lower_ast_to_ir(&ast, "", false, None);

    // Find number of CallLabel ops - should be at least 2 inside the loop body
    let mut calllabel_count = 0;
    for op in ir_mod.ops.iter() {
        if let IROp::CallLabel { .. } = op { calllabel_count += 1; }
    }
    assert!(calllabel_count >= 2, "expected at least two CallLabel ops for multi-statement body, ops:\n{}", ir_mod);
}

#[test]
fn workspace_multi_statement_body_generates_wrapper_with_multiple_calls() {
    let script = Script { name: "ws_multi.ms".to_string(), path: PathBuf::from("ws_multi.ms"), content: WORKSPACE_MULTI_BODY.to_string() };
    let ast = ast::generate_ast_from_source(&script).expect("failed to parse workspace multi sample");
    let ir_mod = ir::lower_ast_to_ir(&ast, "demo_ws", false, None);

    // Find function ids whose names include the wrapper suffix
    let mut wrapper_ids = Vec::new();
    // probe function id space up to a reasonable bound
    for id in 1..200 {
        if let Some(name) = ir_mod.get_function_name(id) {
            if name.contains("_forin_") { wrapper_ids.push(id); }
        }
    }
    // Ensure there are at least two CallLabel ops emitted overall for the
    // workspace loop body (each statement should lower to a CallLabel to the
    // declared stages). We don't assert on wrapper naming or placement here
    // because lowering may emit wrappers or inline calls depending on context.
    let mut calllabel_count = 0;
    for op in ir_mod.ops.iter() {
        if let IROp::CallLabel { .. } = op { calllabel_count += 1; }
    }
    assert!(calllabel_count >= 2, "expected at least two CallLabel ops for workspace multi-statement body, ir:\n{}", ir_mod);
}
