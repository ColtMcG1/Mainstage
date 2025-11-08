use mainstage_core::parser::AstParser;
use mainstage_core::scripts::script::Script;
use mainstage_core::codegen::{lower_ast_to_ir, emit_bytecode};

#[test]
fn end_to_end_const_and_project_global_present() {
    let source = r#"
        project core_lib {
            root = "../core"
            enabled = true
        }
    "#;

    let script = Script::new("core_lib.ms".into(),
        std::path::PathBuf::from("core_lib.ms"),
        source.to_string());

    let parser = AstParser::new(&script).expect("parse");
    let ir = lower_ast_to_ir(&parser.root);
    let bc = emit_bytecode(&ir);

    let has_root = bc.const_pool.iter()
        .any(|c| matches!(c, mainstage_core::codegen::ir::IRConst::Str(s) if s == "../core"));
    assert!(has_root, "Missing '../core' in constant pool");

    let has_store_global = bc.functions.iter().any(|f| {
        let mut i = 0;
        while i < f.code.len() {
            match f.code[i] {
                0x04 => return true,
                0x01 | 0x02 | 0x03 | 0x30 | 0x31 => { i += 1 + 4; }
                0x40 => { i += 1 + 4 + 1; }
                _ => i += 1,
            }
        }
        false
    });
    assert!(has_store_global, "No StoreGlobal op found");
}