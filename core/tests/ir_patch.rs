use mainstage_core::ir::module::IrModule;
use mainstage_core::ir::op::IROp;

#[test]
fn patch_unresolved_branch_to_label() {
    let mut m = IrModule::new();
    // emit a placeholder BrFalse at position 0
    m.emit_op(IROp::BrFalse { cond: 1, target: 0 });
    // record unresolved branch pointing to label "L_after"
    m.record_unresolved_branch(0, "L_after".to_string());
    // emit some ops
    m.emit_op(IROp::LConst { dest: 2, value: mainstage_core::ir::value::Value::Null });
    // emit the label we referenced
    m.emit_op(IROp::Label { name: "L_after".to_string() });

    // Now patch
    m.patch_unresolved_branches();

    // Verify that the BrFalse at 0 now targets the label index (which should be 2)
    if let IROp::BrFalse { cond: _, target } = &m.get_ops()[0] {
        assert_eq!(*target, 2usize);
    } else {
        panic!("expected BrFalse at op 0");
    }
}
