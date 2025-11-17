use crate::parser::ast::AstNode;
use crate::parser::types::AstType;
use super::context::LowerCtx;
use super::expr::lower_expr;
use crate::codegen::op::Op;
use crate::codegen::value::OpValue;

pub fn lower_ast_to_ir(root: &AstNode) -> LowerCtx {
    let mut cx = LowerCtx::new();

    // Pass 1: record scope nodes (do not inline bodies)
    for child in &root.children {
        record_scopes_recursive(&mut cx, child);
    }

    if let Some(ws) = cx.entry_workspace.clone() {
        cx.emit(Op::CallScope { name: ws.clone() });
        cx.note_scope_call(&ws);
    }

    // Pass 2: lower top-level statements (excluding scope body emission)
    for child in &root.children {
        lower_toplevel(&mut cx, child);
    }

    // Emit scope regions at end
    cx.emit_scope_regions(|ctx, node| lower_stmt(ctx, node));

    cx
}

fn record_scopes_recursive(cx: &mut LowerCtx, node: &AstNode) {
    match &node.kind {
        AstType::Workspace { .. }
        | AstType::Project { .. }
        | AstType::Stage { .. }
        | AstType::Task { .. } => {
            cx.record_scope(node);
        }
        _ => {}
    }
    for c in &node.children {
        record_scopes_recursive(cx, c);
    }
}

fn lower_toplevel(cx: &mut LowerCtx, node: &AstNode) {
    match &node.kind {
        AstType::Workspace { .. }
        | AstType::Project { .. }
        | AstType::Stage { .. }
        | AstType::Task { .. } => { /* emitted as regions later */ }

        AstType::Call { target, arguments } => {
            if let AstType::Identifier { name } = &target.kind {
                if cx.scope_names.contains(name.as_ref()) && arguments.is_empty() {
                    cx.emit(Op::CallScope { name: name.to_string() });
                    cx.note_scope_call(name.as_ref());
                    return;
                }
            }
            let _ = lower_expr(cx, node);
        }

        _ => lower_stmt(cx, node),
    }
}

fn lower_stmt(cx: &mut LowerCtx, node: &AstNode) {
    match &node.kind {
        AstType::Assignment => {
            if node.children.len() < 2 { return; }
            let lhs = &node.children[0];
            let rhs = &node.children[1];
            match &lhs.kind {
                // Inside a scope region, treat plain "x = expr" as member write on current scope.
                AstType::Identifier { name: member } => {
                    if let Some(val) = lower_expr(cx, rhs) {
                        if let Some(scope_name) = cx.current_scope.clone() {
                            // Load scope object then MSet
                            let obj = cx.temp();
                            cx.emit(Op::LoadGlobal { target: obj, name: scope_name.clone() });
                            cx.emit(Op::MSet { target: obj, member: member.to_string(), value: val });
                            cx.note_member_init(&scope_name, member.as_ref());
                        } else {
                            // Not in a scope region: regular global/local store
                            if cx.is_root() {
                                cx.emit(Op::StoreGlobal { source: val, name: member.to_string() });
                            } else {
                                let local = cx.lookup_local(member.as_ref()).unwrap_or_else(|| cx.ensure_local(member.as_ref()));
                                cx.emit(Op::StoreLocal { source: val, target: local });
                            }
                        }
                    }
                }

                // Explicit member assignment: container.member = expr
                AstType::Member { target, member } => {
                    let obj = lower_expr(cx, target);
                    if let (Some(o), AstType::Identifier { name: m }) = (obj, &member.kind) {
                        if let Some(v) = lower_expr(cx, rhs) {
                            cx.emit(Op::MSet { target: o, member: m.to_string(), value: v });
                            // If container is a simple identifier, record init for that scope name
                            if let AstType::Identifier { name: container } = &target.kind {
                                cx.note_member_init(container.as_ref(), m.as_ref());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Treat calls to scope names anywhere (not just at top-level) as CallScope
        AstType::Call { target, arguments } => {
            if let AstType::Identifier { name } = &target.kind {
                if cx.scope_names.contains(name.as_ref()) && arguments.is_empty() {
                    cx.emit(Op::CallScope { name: name.to_string() });
                    cx.note_scope_call(name.as_ref());
                    return;
                }
            }
            let _ = lower_expr(cx, node);
        }

        AstType::Return => {
            let val = node.children.get(0).and_then(|c| lower_expr(cx, c));
            cx.emit(Op::Return { value: val });
        }

        AstType::Block => {
            cx.push_frame();
            for c in &node.children { lower_stmt(cx, c); }
            cx.pop_frame();
        }

        AstType::For { .. } => lower_for(cx, node),
        AstType::Forin { .. } => lower_forin(cx, node),
        AstType::While { .. } => lower_while(cx, node),

        _ => {
            let _ = lower_expr(cx, node);
        }
    }
}

// robust for-loop lowering: for <var> in <iterable> { <body> }
fn lower_for(cx: &mut LowerCtx, node: &AstNode) {
    // children: [var, iterable, body] (fallbacks tolerated)
    let (var_node, iterable_node, body_node) = if node.children.len() >= 3 {
        (&node.children[0], &node.children[1], &node.children[2])
    } else if node.children.len() >= 2 {
        (&node.children[0], &node.children[0], &node.children[1])
    } else {
        return;
    };

    let var_name = match &var_node.kind {
        AstType::Identifier { name } => name.as_ref().to_string(),
        _ => "$for_item".to_string(),
    };

    let arr_slot = match lower_expr(cx, iterable_node) { Some(s) => s, None => return };

    // idx local = 0
    let idx_name = format!("$idx#{}", node.id);
    let idx_slot = cx.lookup_local(&idx_name).unwrap_or_else(|| cx.ensure_local(&idx_name));
    let zero = cx.temp();
    cx.emit(Op::LoadConst { target: zero, value: OpValue::Int(0) });
    cx.emit(Op::StoreLocal { source: zero, target: idx_slot });

    // len = Length(arr)
    let len_slot = cx.temp();
    cx.emit(Op::Length { target: len_slot, array: arr_slot });

    // Labels
    let start_label = format!("for.start.{}", node.id);
    let end_label = format!("for.end.{}", node.id);
    cx.emit(Op::Label { name: start_label.clone() });

    // cond: idx < len
    let idx_val = cx.temp();
    cx.emit(Op::LoadLocal { target: idx_val, source: idx_slot });
    let cond = cx.temp();
    cx.emit(Op::Lt { lhs: idx_val, rhs: len_slot, target: cond });
    cx.emit(Op::BrFalse { condition: cond, target: end_label.clone() });

    // item = arr[idx]
    let idx_cur = cx.temp();
    cx.emit(Op::LoadLocal { target: idx_cur, source: idx_slot });
    let item_slot = cx.temp();
    cx.emit(Op::IGet { target: item_slot, source: arr_slot, index: idx_cur });

    // bind loop variable
    let loop_var_slot = cx.lookup_local(&var_name).unwrap_or_else(|| cx.ensure_local(&var_name));
    cx.emit(Op::StoreLocal { source: item_slot, target: loop_var_slot });

    // body
    lower_stmt(cx, body_node);

    // idx++
    let idx_before = cx.temp();
    cx.emit(Op::LoadLocal { target: idx_before, source: idx_slot });
    let one = cx.temp();
    cx.emit(Op::LoadConst { target: one, value: OpValue::Int(1) });
    let idx_next = cx.temp();
    cx.emit(Op::Add { lhs: idx_before, rhs: one, target: idx_next });
    cx.emit(Op::StoreLocal { source: idx_next, target: idx_slot });

    cx.emit(Op::Jump { target: start_label });
    cx.emit(Op::Label { name: end_label });
}

// forin lowering: same as for; alias to lower_for with tolerant shape
fn lower_forin(cx: &mut LowerCtx, node: &AstNode) {
    // Typically also [var, iterable, body]; reuse lower_for
    lower_for(cx, node);
}

// while lowering: while <cond> { <body> }
fn lower_while(cx: &mut LowerCtx, node: &AstNode) {
    // children: [cond, body] preferred
    if node.children.is_empty() { return; }
    let cond_node = &node.children[0];
    let body_node = node.children.get(1);

    let start = format!("while.start.{}", node.id);
    let end = format!("while.end.{}", node.id);
    cx.emit(Op::Label { name: start.clone() });

    if let Some(cond_slot) = lower_expr(cx, cond_node) {
        cx.emit(Op::BrFalse { condition: cond_slot, target: end.clone() });
        if let Some(b) = body_node {
            lower_stmt(cx, b);
        }
        cx.emit(Op::Jump { target: start });
        cx.emit(Op::Label { name: end });
    }
}