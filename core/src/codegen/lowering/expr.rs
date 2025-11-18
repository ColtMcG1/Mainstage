use crate::parser::ast::AstNode;
use crate::parser::types::{AstType, BinaryOperator};
use crate::codegen::value::{literal_from_node, OpValue};
use crate::codegen::op::Op;
use crate::codegen::slot::Slot;
use crate::report;
use super::context::LowerCtx;

pub fn lower_expr(cx: &mut LowerCtx, node: &AstNode) -> Option<Slot> {
    match &node.kind {
        AstType::Integer { .. }
        | AstType::Float { .. }
        | AstType::Bool { .. }
        | AstType::Str { .. }
        | AstType::Null => {
            if let Some(v) = literal_from_node(node) {
                let t = cx.temp();
                cx.emit(Op::LoadConst { target: t, value: v });
                Some(t)
            } else { None }
        }
        AstType::Array => {
            let target = cx.temp();
            cx.emit(Op::NewArray { target, size: node.children.len() });
            for (i, ch) in node.children.iter().enumerate() {
                if let Some(val_slot) = lower_expr(cx, ch) {
                    let idx = cx.temp();
                    cx.emit(Op::LoadConst { target: idx, value: OpValue::Int(i as i64) });
                    cx.emit(Op::ISet { target, index: idx, value: val_slot });
                }
            }
            Some(target)
        }

        AstType::Identifier { name } => {
            let n = name.as_ref();

            if cx.scope_names.contains(n) && !cx.has_called_scope(n) {
                cx.emit(Op::CallScope { name: n.to_string() });
                cx.note_scope_call(n);
            }

            if let Some(loc) = cx.lookup_local(n) {
                let t = cx.temp();
                cx.emit(Op::LoadLocal { target: t, source: loc });
                return Some(t);
            }

            if let Some(scope_name) = cx.current_scope.as_ref() {
                let scope_name_cloned = scope_name.clone();
                if cx.is_member_initialized(&scope_name_cloned, n) {
                    let obj = cx.temp();
                    cx.emit(Op::LoadGlobal { target: obj, name: scope_name_cloned.clone() });
                    let t = cx.temp();
                    cx.emit(Op::MGet { target: t, source: obj, member: n.to_string() });
                    return Some(t);
                }
            }

            let t = cx.temp();
            cx.emit(Op::LoadGlobal { target: t, name: n.to_string() });
            Some(t)
        }
        AstType::BinaryOp { op, left, right } => {
            let l = lower_expr(cx, left)?;
            let r = lower_expr(cx, right)?;
            let t = cx.temp();
            match op {
                BinaryOperator::Add => cx.emit(Op::Add { lhs: l, rhs: r, target: t }),
                BinaryOperator::Sub => cx.emit(Op::Sub { lhs: l, rhs: r, target: t }),
                BinaryOperator::Mul => cx.emit(Op::Mul { lhs: l, rhs: r, target: t }),
                BinaryOperator::Div => cx.emit(Op::Div { lhs: l, rhs: r, target: t }),
                BinaryOperator::Eq => cx.emit(Op::Eq { lhs: l, rhs: r, target: t }),
                BinaryOperator::Ne => cx.emit(Op::Ne { lhs: l, rhs: r, target: t }),
                BinaryOperator::Lt => cx.emit(Op::Lt { lhs: l, rhs: r, target: t }),
                BinaryOperator::Le => cx.emit(Op::Le { lhs: l, rhs: r, target: t }),
                BinaryOperator::Gt => cx.emit(Op::Gt { lhs: l, rhs: r, target: t }),
                BinaryOperator::Ge => cx.emit(Op::Ge { lhs: l, rhs: r, target: t }),
            }
            Some(t)
        }
        AstType::Call { target, arguments } => {
            // Builtins
            if let AstType::Identifier { name } = &target.kind {
                match name.as_ref() {
                    "say" => {
                        if let Some(arg) = arguments.get(0).and_then(|a| lower_expr(cx, a)) {
                            cx.emit(Op::Say { message: arg });
                        }
                        return None;
                    }
                    "ask" => {
                        if let Some(q) = arguments.get(0).and_then(|a| lower_expr(cx, a)) {
                            let t = cx.temp();
                            cx.emit(Op::Ask { question: q, target: t });
                            return Some(t);
                        }
                        return None;
                    }
                    "read" => {
                        if let Some(loc) = arguments.get(0).and_then(|a| lower_expr(cx, a)) {
                            let t = cx.temp();
                            cx.emit(Op::Read { location: loc, target: t });
                            return Some(t);
                        }
                        return None;
                    }
                    "write" => {
                        if arguments.len() >= 2 {
                            if let (Some(loc), Some(content)) = (lower_expr(cx, &arguments[0]), lower_expr(cx, &arguments[1])) {
                                cx.emit(Op::Write { location: loc, target: content });
                            }
                        }
                        return None;
                    }
                    _ => {}
                }
            }
            // Generic call
            let func_slot = lower_expr(cx, target)?;
            let mut arg_slots = Vec::new();
            for a in arguments {
                if let Some(s) = lower_expr(cx, a) { arg_slots.push(s); }
            }
            let ret = cx.temp();
            cx.emit(Op::Call { target: ret, func: func_slot, args: arg_slots });
            Some(ret)
        }
        AstType::Return => {
            if let Some(v) = node.children.get(0) {
                lower_expr(cx, v)
            } else { None }
        }
        AstType::Member { target, member } => {
            // Auto-init if target is a scope identifier and not yet called (covers demo_stage.test)
            if let (AstType::Identifier { name: container }, AstType::Identifier { .. }) = (&target.kind, &member.kind) {
                let c = container.as_ref();
                if cx.scope_names.contains(c) && !cx.has_called_scope(c) {
                    cx.emit(Op::CallScope { name: c.to_string() });
                    cx.note_scope_call(c);
                }
            }
            let src = lower_expr(cx, target)?;
            if let AstType::Identifier { name: m } = &member.kind {
                let t = cx.temp();
                cx.emit(Op::MGet { target: t, source: src, member: m.to_string() });
                Some(t)
            } else { None }
        }
        _ => {
            report!(
                crate::reports::Level::Error,
                format!("Unsupported expression kind in lowering: {:?}", node.kind),
                Some("Lowering".into()),
                node.span.clone(),
                node.location.clone()
            );
            None
        }
    }
}