use std::path::PathBuf;
use mainstage_core::{ast, ir, script::Script};

#[test]
fn lowering_if_emits_brfalse() {
    let src = r#"
stage f() { if true { return; } }
"#;
    let script = Script { name: "t.ms".to_string(), path: PathBuf::from("t.ms"), content: src.to_string() };
    let ast = ast::generate_ast_from_source(&script).expect("parse");
    let ir_mod = ir::lower_ast_to_ir(&ast, "", false, None);
    // Expect a BrFalse in the lowered IR
    let has_brfalse = ir_mod.get_ops().iter().any(|op| matches!(op, mainstage_core::ir::op::IROp::BrFalse { .. }));
    assert!(has_brfalse, "expected BrFalse in IR");
}

#[test]
fn lowering_while_emits_jump_back_and_brfalse() {
    let src = r#"
stage f() { while true { return; } }
"#;
    let script = Script { name: "t.ms".to_string(), path: PathBuf::from("t.ms"), content: src.to_string() };
    let ast = ast::generate_ast_from_source(&script).expect("parse");
    let ir_mod = ir::lower_ast_to_ir(&ast, "", false, None);
    let has_jump = ir_mod.get_ops().iter().any(|op| matches!(op, mainstage_core::ir::op::IROp::Jump { .. }));
    let has_brfalse = ir_mod.get_ops().iter().any(|op| matches!(op, mainstage_core::ir::op::IROp::BrFalse { .. }));
    assert!(has_jump, "expected Jump in IR for while loop");
    assert!(has_brfalse, "expected BrFalse in IR for while loop");
}

#[test]
fn bytecode_calllabel_has_args() {
    let src = r#"
stage callee(x) { return; }
stage caller() { callee("arg"); }
"#;
    let script = Script { name: "t.ms".to_string(), path: PathBuf::from("t.ms"), content: src.to_string() };
    let ast = ast::generate_ast_from_source(&script).expect("parse");
    let ir_mod = ir::lower_ast_to_ir(&ast, "", false, None);
    let bytes = ir::emit_bytecode(&ir_mod);

    // parse header
    assert!(bytes.len() > 12);
    let op_count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;

    fn read_u32(buf: &[u8], off: &mut usize) -> u32 { let v = u32::from_le_bytes(buf[*off..*off+4].try_into().unwrap()); *off += 4; v }
    fn read_u64(buf: &[u8], off: &mut usize) -> u64 { let v = u64::from_le_bytes(buf[*off..*off+8].try_into().unwrap()); *off += 8; v }
    fn read_string(buf: &[u8], off: &mut usize) -> String { let len = read_u32(buf, off) as usize; let s = String::from_utf8(buf[*off..*off+len].to_vec()).unwrap(); *off += len; s }
    fn skip_value(buf: &[u8], off: &mut usize) {
        let tag = buf[*off]; *off += 1;
        match tag {
            0x01 => { read_u64(buf, off); }
            0x02 => { read_u64(buf, off); }
            0x03 => { *off += 1; }
            0x04 | 0x05 => { let _ = read_string(buf, off); }
            0x06 => { let len = read_u32(buf, off) as usize; for _ in 0..len { skip_value(buf, off); } }
            0x07 => {}
            _ => {}
        }
    }

    let mut found = false;
    let mut i = 0usize;
    let mut off = 12usize;
    while i < op_count && off < bytes.len() {
        let code = bytes[off]; off += 1; i += 1;
        match code {
            0x71 => {
                off += 4; // dest
                off += 4; // label idx
                let argc = read_u32(&bytes, &mut off) as usize;
                if argc >= 1 { found = true; break; }
                off += argc * 4;
            }
            // skip common op encodings conservatively
            0x01 => { let _ = read_u32(&bytes, &mut off); skip_value(&bytes, &mut off); }
            0x02|0x03|0x30|0x31 => { off += 4; }
            0x10..=0x14 | 0x20..=0x27 | 0x42|0x43 => { off += 4+4+4; }
            0x40 => { let _ = read_string(&bytes, &mut off); }
            _ => { /* best-effort skip */ }
        }
    }

    assert!(found, "expected CallLabel opcode with at least one arg in bytecode");
}
