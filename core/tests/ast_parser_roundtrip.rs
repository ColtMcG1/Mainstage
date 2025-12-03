use std::path::PathBuf;
use mainstage_core::{ast, script::Script};

#[test]
fn parse_simple_script_returns_ast() {
    let src = r#"
workspace w { projects = []; }

stage f() { return 42; }
"#;

    let script = Script { name: "p.ms".to_string(), path: PathBuf::from("p.ms"), content: src.to_string() };
    let ast = ast::generate_ast_from_source(&script).expect("failed to parse sample");

    // Basic sanity checks on the AST: expect Script with non-empty body
    match ast.get_kind() {
        mainstage_core::ast::AstNodeKind::Script { body } => {
            assert!(!body.is_empty(), "Script body should contain items");
        }
        other => panic!("Unexpected AST root kind: {:?}", other),
    }
}
