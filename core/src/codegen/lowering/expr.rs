use super::context::LowerCtx;
use crate::codegen::op::Op;
use crate::codegen::slot::Slot;
use crate::codegen::value::{OpValue, literal_from_node};
use crate::parser::ast::AstNode;
use crate::parser::types::{AstType, BinaryOperator};

pub fn lower_expr(cx: &mut LowerCtx, node: &AstNode) -> Option<Slot> {
    match &node.kind {
        // literals
        AstType::Integer { .. } | AstType::Float { .. } | AstType::Bool { .. } | AstType::Str { .. } | AstType::Null => {
            let v = literal_from_node(node)?;
            let t = cx.temp();
            cx.emit(Op::LoadConst { target: t, value: v });
            Some(t)
        }

        AstType::Array => {
            let t = cx.temp();
            cx.emit(Op::NewArray { target: t, size: node.children.len() });
            for (i, ch) in node.children.iter().enumerate() {
                if let Some(v) = lower_expr(cx, ch) {
                    let idx = cx.temp();
                    cx.emit(Op::LoadConst { target: idx, value: OpValue::Int(i as i64) });
                    cx.emit(Op::ISet { target: t, index: idx, value: v });
                }
            }
            Some(t)
        }

        AstType::Identifier { name } => {
            let n = name.as_ref();
            // First reference to a scope -> call it once to init
            if cx.scope_names.contains(n) && !cx.has_called_scope(n) {
                let f = cx.temp();
                cx.emit(Op::LoadConst { target: f, value: OpValue::Str(n.to_string()) });
                let sink = cx.temp();
                cx.emit(Op::Call { target: sink, func: f, args: vec![] });
                cx.note_scope_call(n);
            }
            if let Some(loc) = cx.lookup_local(n) {
                let t = cx.temp();
                cx.emit(Op::LoadLocal { target: t, source: loc });
                return Some(t);
            }
            // inside a scope: treat initialized names as members
            if let Some(scope) = &cx.current_scope {
                if cx.is_member_initialized(scope, n) {
                    let scope_name = scope.clone();
                    let obj = cx.temp();
                    cx.emit(Op::LoadGlobal { target: obj, name: scope_name });
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
            let l = lower_expr(cx, left)?; let r = lower_expr(cx, right)?; let t = cx.temp();
            match op {
                BinaryOperator::Add => cx.emit(Op::Add { lhs: l, rhs: r, target: t }),
                BinaryOperator::Sub => cx.emit(Op::Sub { lhs: l, rhs: r, target: t }),
                BinaryOperator::Mul => cx.emit(Op::Mul { lhs: l, rhs: r, target: t }),
                BinaryOperator::Div => cx.emit(Op::Div { lhs: l, rhs: r, target: t }),
                BinaryOperator::Eq  => cx.emit(Op::Eq  { lhs: l, rhs: r, target: t }),
                BinaryOperator::Ne  => cx.emit(Op::Ne  { lhs: l, rhs: r, target: t }),
                BinaryOperator::Lt  => cx.emit(Op::Lt  { lhs: l, rhs: r, target: t }),
                BinaryOperator::Le  => cx.emit(Op::Le  { lhs: l, rhs: r, target: t }),
                BinaryOperator::Gt  => cx.emit(Op::Gt  { lhs: l, rhs: r, target: t }),
                BinaryOperator::Ge  => cx.emit(Op::Ge  { lhs: l, rhs: r, target: t }),
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

                // Scope call in expression position
                let callee = name.as_ref();
                if cx.scope_names.contains(callee) {
                    let f = cx.temp();
                    cx.emit(Op::LoadConst { target: f, value: OpValue::Str(callee.to_string()) });
                    let mut args = Vec::new();
                    for a in arguments { if let Some(s) = lower_expr(cx, a) { args.push(s); } }
                    let ret = cx.temp();
                    cx.emit(Op::Call { target: ret, func: f, args });
                    return Some(ret);
                }
            }

            // Generic call
            let func = lower_expr(cx, target)?;
            let mut args = Vec::new();
            for a in arguments { if let Some(s) = lower_expr(cx, a) { args.push(s); } }
            let ret = cx.temp();
            cx.emit(Op::Call { target: ret, func, args });
            Some(ret)
        }

        _ => None,
    }
}
