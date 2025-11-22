use super::context::LowerCtx;
use super::expr::lower_expr;
use crate::ir::op::Op;
use crate::ir::value::OpValue;
use crate::parser::ast::AstNode;
use crate::parser::types::AstType;
use crate::parser::AssignOperator;

// Collect scopes from the whole tree
fn record_scopes_recursive(cx: &mut LowerCtx, node: &AstNode) {
    use AstType::*;
    match &node.kind {
        Workspace { .. } | Project { .. } | Stage { .. } | Task { .. } => {
            cx.record_scope(node);
        }
        _ => {}
    }
    for ch in &node.children {
        record_scopes_recursive(cx, ch);
    }
}

pub fn lower_ast_to_ir(root: &AstNode, entry: &str) -> LowerCtx {
    let mut cx = LowerCtx::new();

    // 1) Discover all scopes and entrypoint
    record_scopes_recursive(&mut cx, root);

    if !entry.is_empty() {
        cx.entry = entry.to_string();
    }

    // 2) If an explicit [entrypoint] exists, emit an entry Call and then Halt
    if !cx.entry.is_empty() {
        let f = cx.temp();
        cx.emit(Op::LoadConst { target: f, value: OpValue::Str(cx.entry.clone()) });
        let sink = cx.temp(); // discard return
        cx.emit(Op::Call { target: sink, func: f, args: vec![] });
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

fn lower_block(cx: &mut LowerCtx, node: &AstNode) {
    for ch in &node.children {
        lower_stmt(cx, ch);
    }
}

fn lower_stmt(cx: &mut LowerCtx, node: &AstNode) {
    match &node.kind {
        AstType::Workspace { .. }
        | AstType::Project { .. }
        | AstType::Stage { .. }
        | AstType::Task { .. } => {
            lower_block(cx, node);
        }

        AstType::If { cond, body } => {
            let Some(c) = lower_expr(cx, cond) else { return; };
            let (_, lbl_end) = mk_labels("if", &node.id);
            br_false(cx, c, &lbl_end);
            lower_stmt(cx, body);
            cx.emit(Op::Label { name: lbl_end });
        },

        AstType::IfElse { cond, if_body, else_body } => {
            let Some(c) = lower_expr(cx, cond) else { return; };
            let (lbl_else, lbl_end) = (format!("if.else.{}", node.id), format!("if.end.{}", node.id));
            br_false(cx, c, &lbl_else);
            lower_stmt(cx, if_body);
            cx.emit(Op::Jump { target: lbl_end.clone() });
            cx.emit(Op::Label { name: lbl_else });
            lower_stmt(cx, else_body);
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
            cx.push_frame(false);
            for c in &node.children {
                lower_stmt(cx, c);
            }
            cx.pop_frame();
        }

        AstType::Call { .. } => {
            let _ = lower_expr(cx, node); // side-effects only; discard result
        }

        // Default: expression statement
        _ => {
            for ch in &node.children {
                lower_stmt(cx, ch);
            }
        }
    }
}

// ADD: tiny helpers to remove repetition
fn mk_labels(kind: &str, id: impl std::fmt::Display) -> (String, String) {
    (format!("{kind}.start.{id}"), format!("{kind}.end.{id}"))
}

fn br_false(cx: &mut LowerCtx, cond: crate::ir::slot::Slot, target: &str) {
    cx.emit(Op::BrFalse { condition: cond, target: target.to_string() });
}

// Emits lhs <op> rhs into a fresh temp and returns it (for +=, -=, *=, /= use-cases)
fn emit_binop(
    cx: &mut LowerCtx,
    op: AssignOperator,
    lhs: crate::ir::slot::Slot,
    rhs: crate::ir::slot::Slot,
) -> crate::ir::slot::Slot {
    let out = cx.temp();
    match op {
        AssignOperator::Add => cx.emit(Op::Add { lhs, rhs, target: out }),
        AssignOperator::Sub => cx.emit(Op::Sub { lhs, rhs, target: out }),
        AssignOperator::Mul => cx.emit(Op::Mul { lhs, rhs, target: out }),
        AssignOperator::Div => cx.emit(Op::Div { lhs, rhs, target: out }),
        AssignOperator::Set => unreachable!(),
    }
    out
}

fn lower_assignment(cx: &mut LowerCtx, node: &AstNode) {
    if let AstType::Assignment { op } = &node.kind {
        if node.children.len() < 2 { return; }
        let lhs = &node.children[0];
        let rhs = &node.children[1];
        let Some(rhs_slot) = lower_expr(cx, rhs) else { return; };

        if let AstType::Identifier { name } = &lhs.kind {
            let ident = name.as_ref();
            let slot = cx.lookup_local(ident).unwrap_or_else(|| cx.ensure_local(ident));

            if matches!(op, AssignOperator::Set) {
                cx.emit(Op::StoreLocal { source: rhs_slot, target: slot });
            } else {
                let cur = cx.temp();
                cx.emit(Op::LoadLocal { target: cur, source: slot });
                let result = emit_binop(cx, (*op).clone(), cur, rhs_slot);
                cx.emit(Op::StoreLocal { source: result, target: slot });
            }
        }
    }
}

// While lowering: children[0]=cond, children[1]=body
fn lower_while(cx: &mut LowerCtx, node: &AstNode) {
    if node.children.is_empty() { return; }
    let cond = &node.children[0];
    let body = node.children.get(1);
    let (l_start, l_end) = mk_labels("while", &node.id);

    cx.emit(Op::Label { name: l_start.clone() });
    if let Some(c) = lower_expr(cx, cond) {
        br_false(cx, c, &l_end);
        if let Some(b) = body { lower_stmt(cx, b); }
        cx.emit(Op::Jump { target: l_start });
        cx.emit(Op::Label { name: l_end });
    }
}

// for i in <iterable> { body }
fn lower_for_in(cx: &mut LowerCtx, node: &AstNode) {
    use AstType::*;
    let (var_name, iterable_node, body_opt) = match &node.kind {
        Forin { iden, iter, body } => (iden.as_ref().to_string(), iter.as_ref(), Some(body.as_ref())),
        _ => return,
    };

    let Some(arr) = lower_expr(cx, iterable_node) else { return; };

    // idx = 0
    let idx_name = format!("$idx#{}", node.id);
    let idx_slot = cx.lookup_local(&idx_name).unwrap_or_else(|| cx.ensure_local(&idx_name));
    let z = cx.temp();
    cx.emit(Op::LoadConst { target: z, value: OpValue::Int(0) });
    cx.emit(Op::StoreLocal { source: z, target: idx_slot });

    // len = Length(arr)
    let len = cx.temp();
    cx.emit(Op::Length { target: len, array: arr });

    // labels
    let (l_start, l_end) = mk_labels("for", &node.id);
    cx.emit(Op::Label { name: l_start.clone() });

    // cond: idx < len
    let idx_val = cx.temp();
    cx.emit(Op::LoadLocal { target: idx_val, source: idx_slot });
    let cond = cx.temp();
    cx.emit(Op::Lt { lhs: idx_val, rhs: len, target: cond });
    br_false(cx, cond, &l_end);

    // item = arr[idx]
    let idx_cur = cx.temp();
    cx.emit(Op::LoadLocal { target: idx_cur, source: idx_slot });
    let item = cx.temp();
    cx.emit(Op::IGet { target: item, source: arr, index: idx_cur });

    // bind loop var
    let var_slot = cx.lookup_local(&var_name).unwrap_or_else(|| cx.ensure_local(&var_name));
    cx.emit(Op::StoreLocal { source: item, target: var_slot });

    // body
    if let Some(b) = body_opt { lower_stmt(cx, b); }

    // idx++
    cx.emit(Op::Inc { target: idx_slot });

    cx.emit(Op::Jump { target: l_start });
    cx.emit(Op::Label { name: l_end });
}

// for i = <start> to <limit> { body }  (inclusive)
fn lower_for_to(cx: &mut LowerCtx, node: &AstNode) {
    use AstType::*;
    let (var_name, start_node, limit_node, body_node) = match &node.kind {
        Forto { init, limt, body } => {
            if init.children.len() < 2 { return; }
            let lhs = &init.children[0];
            let rhs = &init.children[1];
            let Identifier { name } = &lhs.kind else { return; };
            (name.as_ref().to_string(), rhs, limt.as_ref(), body.as_ref())
        }
        _ => return,
    };

    let Some(start) = lower_expr(cx, start_node) else { return; };
    let Some(limit) = lower_expr(cx, limit_node) else { return; };

    // i = start
    let i_slot = cx.lookup_local(&var_name).unwrap_or_else(|| cx.ensure_local(&var_name));
    cx.emit(Op::StoreLocal { source: start, target: i_slot });

    // labels
    let (l_start, l_end) = mk_labels("for", &node.id);
    cx.emit(Op::Label { name: l_start.clone() });

    // cond: i <= limit
    let i_now = cx.temp();
    cx.emit(Op::LoadLocal { target: i_now, source: i_slot });
    let cond = cx.temp();
    cx.emit(Op::Le { lhs: i_now, rhs: limit, target: cond });
    br_false(cx, cond, &l_end);

    // body
    lower_stmt(cx, body_node);

    // i++
    cx.emit(Op::Inc { target: i_slot });

    cx.emit(Op::Jump { target: l_start });
    cx.emit(Op::Label { name: l_end });
}
