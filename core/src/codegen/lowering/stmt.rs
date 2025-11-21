use super::context::LowerCtx;
use super::expr::lower_expr;
use crate::codegen::op::Op;
use crate::codegen::value::OpValue;
use crate::parser::ast::AstNode;
use crate::parser::types::AstType;

// Collect scopes from the whole tree
fn record_scopes_recursive(cx: &mut LowerCtx, node: &AstNode) {
    cx.record_scope(node);
    for c in &node.children {
        record_scopes_recursive(cx, c);
    }
}

pub fn lower_ast_to_ir(root: &AstNode) -> LowerCtx {
    let mut cx = LowerCtx::new();

    // 1) Discover all scopes and entrypoint
    record_scopes_recursive(&mut cx, root);

    // 2) If an explicit [entrypoint] exists, emit an entry Call and then Halt
    if let Some(entry) = cx.entry.clone() {
        let f = cx.temp();
        cx.emit(Op::LoadConst {
            target: f,
            value: OpValue::Str(entry.clone()),
        });
        let sink = cx.temp(); // discard return
        cx.emit(Op::Call {
            target: sink,
            func: f,
            args: vec![],
        });
        cx.emit(Op::Halt);
    }

    // 3) Lower any non-scope top-level statements if needed (optional)
    for child in &root.children {
        lower_toplevel(&mut cx, child);
    }

    // 4) Emit scope regions; entry returns to Halt above
    cx.emit_scope_regions(|ctx, n| lower_stmt(ctx, n));

    cx
}

fn lower_toplevel(cx: &mut LowerCtx, node: &AstNode) {
    // Treat all top-level non-scope statements as statements
    match &node.kind {
        AstType::Workspace { .. }
        | AstType::Project { .. }
        | AstType::Stage { .. }
        | AstType::Task { .. } => { /* regions emitted later */ }
        _ => lower_stmt(cx, node),
    }
}

fn lower_stmt(cx: &mut LowerCtx, node: &AstNode) {
    match &node.kind {
        AstType::If { cond, body } => {
            // Lower condition
            let c = match lower_expr(cx, cond) {
                Some(s) => s,
                None => return,
            };
            let lbl_end = format!("if.end.{}", node.id);

            // Branch if false
            cx.emit(Op::BrFalse { condition: c, target: lbl_end.clone() });
            // If branch
            lower_stmt(cx, body);
            // End
            cx.emit(Op::Label { name: lbl_end });
        },
        AstType::IfElse { cond, if_body, else_body } => {
            // Lower condition
            let c = match lower_expr(cx, cond) {
                Some(s) => s,
                None => return,
            };
            let lbl_else = format!("if.else.{}", node.id);
            let lbl_end  = format!("if.end.{}", node.id);

            // Branch if false
            cx.emit(Op::BrFalse { condition: c, target: lbl_else.clone() });
            // If branch
            lower_stmt(cx, if_body);
            cx.emit(Op::Jump { target: lbl_end.clone() });

            // Else branch
            cx.emit(Op::Label { name: lbl_else });
            lower_stmt(cx, else_body);

            // End
            cx.emit(Op::Label { name: lbl_end });
        }
        // Assignment
        AstType::Assignment { .. } => lower_assignment(cx, node),
        // While
        AstType::While { .. } => lower_while(cx, node),

        // For-in and For-to handled separately
        AstType::Forin { .. } => lower_for_in(cx, node),
        AstType::Forto { .. } => lower_for_to(cx, node),

        // Return (value optional)
        AstType::Return => {
            let val = node.children.get(0).and_then(|c| lower_expr(cx, c));
            cx.emit(Op::Return { value: val });
        }

        // Block
        AstType::Block => {
            cx.push_frame();
            for c in &node.children {
                lower_stmt(cx, c);
            }
            cx.pop_frame();
        }

        // Default: expression statement
        _ => {
            let _ = lower_expr(cx, node);
        }
    }
}


fn lower_assignment(cx: &mut LowerCtx, node: &AstNode) {
    if let AstType::Assignment { op } = &node.kind {
        if node.children.len() < 2 {
            return;
        }
        let lhs = &node.children[0];
        let rhs = &node.children[1];
        let rhs_slot = match lower_expr(cx, rhs) {
            Some(s) => s,
            None => return,
        };

        if let AstType::Identifier { name } = &lhs.kind {
            let ident = name.as_ref();
            // resolve storage slot (local/global/member)
            let slot = cx
                .lookup_local(ident)
                .unwrap_or_else(|| cx.ensure_local(ident));

            if matches!(op, crate::parser::AssignOperator::Set) {
                cx.emit(Op::StoreLocal {
                    source: rhs_slot,
                    target: slot,
                });
            } else {
                // Load current value
                let cur = cx.temp();
                cx.emit(Op::LoadLocal {
                    target: cur,
                    source: slot,
                });
                let result = cx.temp();
                match op {
                    crate::parser::AssignOperator::Add => cx.emit(Op::Add {
                        lhs: cur,
                        rhs: rhs_slot,
                        target: result,
                    }),
                    crate::parser::AssignOperator::Sub => cx.emit(Op::Sub {
                        lhs: cur,
                        rhs: rhs_slot,
                        target: result,
                    }),
                    crate::parser::AssignOperator::Mul => cx.emit(Op::Mul {
                        lhs: cur,
                        rhs: rhs_slot,
                        target: result,
                    }),
                    crate::parser::AssignOperator::Div => cx.emit(Op::Div {
                        lhs: cur,
                        rhs: rhs_slot,
                        target: result,
                    }),
                    crate::parser::AssignOperator::Set => unreachable!(),
                }
                cx.emit(Op::StoreLocal {
                    source: result,
                    target: slot,
                });
            }
        }
    }
}

// While lowering: children[0]=cond, children[1]=body
fn lower_while(cx: &mut LowerCtx, node: &AstNode) {
    if node.children.is_empty() {
        return;
    }
    let cond = &node.children[0];
    let body = node.children.get(1);

    let l_start = format!("while.start.{}", node.id);
    let l_end = format!("while.end.{}", node.id);

    cx.emit(Op::Label {
        name: l_start.clone(),
    });
    if let Some(c) = lower_expr(cx, cond) {
        cx.emit(Op::BrFalse {
            condition: c,
            target: l_end.clone(),
        });
        if let Some(b) = body {
            lower_stmt(cx, b);
        }
        cx.emit(Op::Jump { target: l_start });
        cx.emit(Op::Label { name: l_end });
    }
}

// for i in <iterable> { body }
fn lower_for_in(cx: &mut LowerCtx, node: &AstNode) {
    use AstType::*;
    let (var_name, iterable_node, body_opt) = match &node.kind {
        Forin { iden, iter, body } => (
            iden.as_ref().to_string(),
            iter.as_ref(),
            Some(body.as_ref()),
        ),
        _ => return,
    };

    let arr = match lower_expr(cx, iterable_node) {
        Some(s) => s,
        None => return,
    };

    // idx = 0
    let idx_name = format!("$idx#{}", node.id);
    let idx_slot = cx
        .lookup_local(&idx_name)
        .unwrap_or_else(|| cx.ensure_local(&idx_name));
    let z = cx.temp();
    cx.emit(Op::LoadConst {
        target: z,
        value: OpValue::Int(0),
    });
    cx.emit(Op::StoreLocal {
        source: z,
        target: idx_slot,
    });

    // len = Length(arr)
    let len = cx.temp();
    cx.emit(Op::Length {
        target: len,
        array: arr,
    });

    // labels
    let l_start = format!("for.start.{}", node.id);
    let l_end = format!("for.end.{}", node.id);
    cx.emit(Op::Label {
        name: l_start.clone(),
    });

    // cond: idx < len
    let idx_val = cx.temp();
    cx.emit(Op::LoadLocal {
        target: idx_val,
        source: idx_slot,
    });
    let cond = cx.temp();
    cx.emit(Op::Lt {
        lhs: idx_val,
        rhs: len,
        target: cond,
    });
    cx.emit(Op::BrFalse {
        condition: cond,
        target: l_end.clone(),
    });

    // item = arr[idx]
    let idx_cur = cx.temp();
    cx.emit(Op::LoadLocal {
        target: idx_cur,
        source: idx_slot,
    });
    let item = cx.temp();
    cx.emit(Op::IGet {
        target: item,
        source: arr,
        index: idx_cur,
    });

    // bind loop var
    let var_slot = cx
        .lookup_local(&var_name)
        .unwrap_or_else(|| cx.ensure_local(&var_name));
    cx.emit(Op::StoreLocal {
        source: item,
        target: var_slot,
    });

    // body
    if let Some(b) = body_opt {
        lower_stmt(cx, b);
    }

    // idx++
    cx.emit(Op::Inc { target: idx_slot });

    cx.emit(Op::Jump { target: l_start });
    cx.emit(Op::Label { name: l_end });
}

// for i = <start> to <limit> { body }  (inclusive)
fn lower_for_to(cx: &mut LowerCtx, node: &AstNode) {
    use AstType::*;
    let (var_name, start_node, limit_node, body_node) = match &node.kind {
        // Forto carries: init (Assignment), limt (end expr), body
        Forto { init, limt, body } => {
            if init.children.len() < 2 {
                return;
            }
            let lhs = &init.children[0];
            let rhs = &init.children[1];
            let var_name = if let Identifier { name } = &lhs.kind {
                name.as_ref().to_string()
            } else {
                return;
            };
            (var_name, rhs, limt.as_ref(), body.as_ref())
        }
        _ => return,
    };

    // Evaluate start and limit
    let start = match lower_expr(cx, start_node) {
        Some(s) => s,
        None => return,
    };
    let limit = match lower_expr(cx, limit_node) {
        Some(s) => s,
        None => return,
    };

    // i = start
    let i_slot = cx
        .lookup_local(&var_name)
        .unwrap_or_else(|| cx.ensure_local(&var_name));
    cx.emit(Op::StoreLocal {
        source: start,
        target: i_slot,
    });

    // labels
    let l_start = format!("for.start.{}", node.id);
    let l_end = format!("for.end.{}", node.id);
    cx.emit(Op::Label {
        name: l_start.clone(),
    });

    // cond: i <= limit
    let i_now = cx.temp();
    cx.emit(Op::LoadLocal {
        target: i_now,
        source: i_slot,
    });
    let cond = cx.temp();
    cx.emit(Op::Le {
        lhs: i_now,
        rhs: limit,
        target: cond,
    });
    cx.emit(Op::BrFalse {
        condition: cond,
        target: l_end.clone(),
    });

    // body
    lower_stmt(cx, body_node);

    // i++
    cx.emit(Op::Inc { target: i_slot });

    cx.emit(Op::Jump { target: l_start });
    cx.emit(Op::Label { name: l_end });
}
