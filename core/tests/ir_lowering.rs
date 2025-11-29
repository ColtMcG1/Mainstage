use mainstage_core::ir::{ module::IrModule, lower::function_builder::FunctionBuilder, op::IROp, value::Value };
use mainstage_core::ir::lower::lower_expr;
use mainstage_core::ast::node::AstNode;
use mainstage_core::ast::kind::AstNodeKind;
use mainstage_core::ir::lower::lowering_context::LoweringContext;

#[test]
fn function_builder_allocs_and_finalize() {
    let mut fb = FunctionBuilder::new();
    let r1 = fb.alloc_reg();
    let _ = fb.alloc_reg();
    let l = fb.get_or_create_local("x");
    assert_eq!(l, 0);
    assert_eq!(r1, 0);
    fb.emit_op(IROp::LConst { dest: r1, value: Value::Int(7) });

    let mut module = IrModule::new();
    fb.finalize_into(&mut module);
    assert_eq!(module.len(), 1);
    assert_eq!(module.get_ops()[0], IROp::LConst { dest: 0, value: Value::Int(7) });
}

#[test]
fn lower_expr_integer_with_builder_emits_lconst_in_builder() {
    let node = AstNode::new(AstNodeKind::Integer { value: 123 }, None, None);
    let mut module = IrModule::new();
    let mut fb = FunctionBuilder::new();
    let ctx = LoweringContext::new();

    let reg = lower_expr::lower_expr_to_reg_with_builder(&node, &mut module, &ctx, Some(&mut fb));
    // reg should be allocated inside builder and builder should contain one op
    assert_eq!(reg, 0);
    assert_eq!(fb.current_len(), 1);
    assert_eq!(fb.ops[0], IROp::LConst { dest: 0, value: Value::Int(123) });
}
