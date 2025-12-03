use mainstage_core::ir::{ module::IrModule, op::IROp, value::Value };

// Build a small synthetic IR that mirrors the shape:
//   r0 = LConst Str("f")
//   r1 = ArrayNew [r0]
//   r2 = LConst Int(0)
//   r3 = ArrayGet r1[r2]
//   r4 = PluginCall dest <- p.f(r3)
//   r5 = LConst Symbol("say")
//   r6 = Call r5(r4)
// Run optimizer and assert that producers for plugin args/results remain.

#[test]
fn optimizer_preserves_plugin_returns() {
    let mut ir_mod = IrModule::new();

    let r0 = ir_mod.alloc_reg();
    ir_mod.emit_op(IROp::LConst { dest: r0, value: Value::Str("fileA".to_string()) });

    let r1 = ir_mod.alloc_reg();
    ir_mod.emit_op(IROp::ArrayNew { dest: r1, elems: vec![r0] });

    let r2 = ir_mod.alloc_reg();
    ir_mod.emit_op(IROp::LConst { dest: r2, value: Value::Int(0) });

    let r3 = ir_mod.alloc_reg();
    ir_mod.emit_op(IROp::ArrayGet { dest: r3, array: r1, index: r2 });

    let r4 = ir_mod.alloc_reg();
    ir_mod.emit_op(IROp::PluginCall { dest: Some(r4), plugin_name: "p".to_string(), func_name: "f".to_string(), args: vec![r3] });

    let r5 = ir_mod.alloc_reg();
    ir_mod.emit_op(IROp::LConst { dest: r5, value: Value::Symbol("say".to_string()) });
    let r6 = ir_mod.alloc_reg();
    ir_mod.emit_op(IROp::Call { dest: r6, func: r5, args: vec![r4] });

    // sanity check: plugin call present
    assert!(ir_mod.ops.iter().any(|op| matches!(op, IROp::PluginCall { .. })), "expected plugincall");

    // Run optimizer
    mainstage_core::ir::opt::optimize(&mut ir_mod);

    // find the plugin call and verify:
    // - the plugin-call argument has a producer before the plugin call
    // - any consumer of the plugin-call dest has a producer before that consumer
    let mut ok = false;
    for (i, op) in ir_mod.ops.iter().enumerate() {
        if let IROp::PluginCall { dest: Some(d), args, .. } = op {
            // ensure there exists an earlier op that writes the arg
            let arg = args[0];
            let arg_has_producer = ir_mod.ops.iter().take(i).any(|p| match p {
                IROp::LConst { dest, .. } | IROp::ArrayNew { dest, .. } | IROp::GetProp { dest, .. } | IROp::ArrayGet { dest, .. } => *dest == arg,
                _ => false,
            });

            // find consumers of plugin dest and ensure they have a producer
            let dest_consumed_ok = ir_mod.ops.iter().enumerate().any(|(j, p)| match p {
                IROp::Call { dest: _, func: _, args } => {
                    if args.iter().any(|a| *a == *d) {
                        // check for any prior producer for the dest `d` before the consumer
                        ir_mod.ops.iter().take(j).any(|pp| {
                            match pp {
                                IROp::PluginCall { dest: Some(pd), .. } => *pd == *d,
                                IROp::LConst { dest, .. } => *dest == *d,
                                IROp::ArrayNew { dest, .. } => *dest == *d,
                                IROp::Call { dest, .. } => *dest == *d,
                                IROp::CallLabel { dest, .. } => *dest == *d,
                                IROp::GetProp { dest, .. } => *dest == *d,
                                _ => false,
                            }
                        })
                    } else { false }
                }
                _ => false
            });

            if arg_has_producer && dest_consumed_ok { ok = true; break; }
        }
    }
    assert!(ok, "optimizer removed plugin-call producers");
}
