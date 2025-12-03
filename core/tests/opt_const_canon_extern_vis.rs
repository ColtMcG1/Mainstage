use mainstage_core::ir::{ module::IrModule, op::IROp, value::Value };

#[test]
fn canonicalize_updates_externally_visible_regs() {
    // Build an IR with duplicate LConst values emitted to two registers
    let mut ir = IrModule::new();

    let r0 = ir.alloc_reg();
    ir.emit_op(IROp::LConst { dest: r0, value: Value::Int(42) });

    let r1 = ir.alloc_reg();
    ir.emit_op(IROp::LConst { dest: r1, value: Value::Int(42) });

    // Mark the *duplicate* register as externally visible (simulating
    // a plugin call or other external observation of r1).
    ir.mark_externally_visible(r1);

    // Sanity: both LConst present and externally set contains r1
    assert!(ir.ops.iter().any(|o| matches!(o, IROp::LConst { dest, .. } if *dest == r0)));
    assert!(ir.ops.iter().any(|o| matches!(o, IROp::LConst { dest, .. } if *dest == r1)));
    assert!(ir.get_externally_visible().contains(&r1));

    // Run public optimizer (which includes const_canon)
    mainstage_core::ir::opt::optimize(&mut ir);

    // After canonicalization, the duplicate LConst should be removed
    // and the externally-visible set should refer to the canonical reg.
    // Debug dump of ops after optimization
    eprintln!("IR ops after optimize:");
    for (i, op) in ir.ops.iter().enumerate() { eprintln!("{}: {:?}", i, op); }
    eprintln!("externally_visible: {:?}", ir.get_externally_visible());
    let vis: Vec<usize> = ir.get_externally_visible().iter().copied().collect();
    // There should be at least one visible entry, and it should be r0 (the first LConst)
    assert!(vis.contains(&r0), "externally-visible set should contain canonical reg r0");
    assert!(!vis.contains(&r1), "externally-visible set should not contain the remapped reg r1");

    // We only assert metadata correctness here: the externally-visible set
    // must have been rewritten to the canonical register and must not
    // contain the remapped original register.
}
