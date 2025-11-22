use super::context::LowerCtx;
use crate::codegen::value::{OpValue, literal_from_node};
use crate::codegen::{Op, Slot};
use crate::parser::types::BinaryOperator;
use crate::parser::{AstNode, AstType};
use crate::semantic::builtin::{BUILTIN_FUNCS, BUILTIN_METHODS, BuiltinIntrinsic};

fn lower_literal(cx: &mut LowerCtx, node: &AstNode) -> Option<Slot> {
    let v = literal_from_node(node)?;
    let t = cx.temp();
    cx.emit(Op::LoadConst {
        target: t,
        value: v,
    });
    Some(t)
}

fn lower_array(cx: &mut LowerCtx, node: &AstNode) -> Option<Slot> {
    let t = cx.temp();
    cx.emit(Op::NewArray {
        target: t,
        size: node.children.len(),
    });
    for (i, ch) in node.children.iter().enumerate() {
        if let Some(v) = lower_expr(cx, ch) {
            let idx = cx.temp();
            cx.emit(Op::LoadConst {
                target: idx,
                value: OpValue::Int(i as i64),
            });
            cx.emit(Op::ISet {
                target: t,
                index: idx,
                value: v,
            });
        }
    }
    Some(t)
}

fn collect_args(cx: &mut LowerCtx, arguments: &[AstNode]) -> Vec<Slot> {
    arguments.iter().filter_map(|a| lower_expr(cx, a)).collect()
}

fn lower_builtin_intrinsic(cx: &mut LowerCtx, name: &str, arguments: &[AstNode]) -> Option<Slot> {
    let Some(def) = BUILTIN_FUNCS.get(name) else {
        return None;
    };
    match def.intrinsic {
        Some(BuiltinIntrinsic::Say) => {
            if let Some(arg) = arguments.get(0).and_then(|a| lower_expr(cx, a)) {
                cx.emit(Op::Say { message: arg });
            }
            None
        }
        Some(BuiltinIntrinsic::Ask) => {
            let out = cx.temp();
            let q = arguments
                .get(0)
                .and_then(|a| lower_expr(cx, a))
                .unwrap_or_else(|| {
                    let tmp = cx.temp();
                    cx.emit(Op::LoadConst {
                        target: tmp,
                        value: OpValue::Str(String::new()),
                    });
                    tmp
                });
            cx.emit(Op::Ask {
                question: q,
                target: out,
            });
            Some(out)
        }
        Some(BuiltinIntrinsic::Read) => {
            if let Some(loc) = arguments.get(0).and_then(|a| lower_expr(cx, a)) {
                let out = cx.temp();
                cx.emit(Op::Read {
                    location: loc,
                    target: out,
                });
                Some(out)
            } else {
                None
            }
        }
        Some(BuiltinIntrinsic::Write) => {
            if let (Some(loc), Some(val)) = (
                arguments.get(0).and_then(|a| lower_expr(cx, a)),
                arguments.get(1).and_then(|a| lower_expr(cx, a)),
            ) {
                cx.emit(Op::Write {
                    location: loc,
                    target: val,
                });
            }
            None
        }
        _ => None,
    }
}

fn lower_scope_call(cx: &mut LowerCtx, name: &str, args: Vec<Slot>) -> Slot {
    let f = cx.temp();
    cx.emit(Op::LoadConst {
        target: f,
        value: OpValue::Str(name.to_string()),
    });
    let ret = cx.temp();
    cx.emit(Op::Call {
        target: ret,
        func: f,
        args,
    });
    ret
}

fn lower_member_call(
    cx: &mut LowerCtx,
    recv: &AstNode,
    member: &AstNode,
    arguments: &[AstNode],
) -> Option<Slot> {
    let AstType::Identifier { name: m } = &member.kind else {
        return None;
    };
    let mem_name = m.as_ref();
    // Builtin method dispatch
    if let Some(meta) = BUILTIN_METHODS.get(mem_name) {
        match meta.name {
            "length" => {
                let recv_slot = lower_expr(cx, recv)?;
                let out = cx.temp();
                cx.emit(Op::Length {
                    target: out,
                    array: recv_slot,
                });
                return Some(out);
            }
            _ => {}
        }
    }
    let args = collect_args(cx, arguments);
    let ret = lower_scope_call(cx, mem_name, args);
    Some(ret)
}

fn lower_identifier(cx: &mut LowerCtx, name: &str) -> Option<Slot> {
    if cx.scope_names.contains(name) && !cx.has_called_scope(name) {
        let param_len = cx.param_names.get(name).map_or(0, |v| v.len());
        if param_len == 0 {
            let _ = lower_scope_call(cx, name, vec![]);
            cx.note_scope_call(name);
            return None;
        }
    }
    if let Some(slot) = cx.lookup_local(name) {
        let t = cx.temp();
        cx.emit(Op::LoadLocal {
            target: t,
            source: slot,
        });
        return Some(t);
    }
    None
}

pub fn lower_expr(cx: &mut LowerCtx, node: &AstNode) -> Option<Slot> {
    match &node.kind {
        AstType::Integer { .. }
        | AstType::Float { .. }
        | AstType::Bool { .. }
        | AstType::Str { .. }
        | AstType::Null => lower_literal(cx, node),

        AstType::Array => lower_array(cx, node),

        AstType::Identifier { name } => lower_identifier(cx, name.as_ref()),

        AstType::BinaryOp { .. } => lower_binary(cx, node),

        AstType::Call { target, arguments } => {
            // Check for intrinsic functions first `identifier(args...)` else check for scope(stage/task) call `scope(args...)`
            if let AstType::Identifier { name } = &target.kind {
                let callee = name.as_ref();
                if let Some(intrinsic) = lower_builtin_intrinsic(cx, callee, arguments) {
                    return Some(intrinsic);
                } else if cx.scope_names.contains(callee) {
                    let args = collect_args(cx, arguments);
                    let ret = lower_scope_call(cx, callee, args);
                    return Some(ret);
                } else {
                    // Fallthrough to member call if not intrinsic or scope call
                }
            }
            // Member call `container.member(args...)`
            if let AstType::Member {
                target: recv,
                member,
            } = &target.kind
            {
                return lower_member_call(cx, recv, member, arguments);
            }
            // General call `func(args...)`
            let func = lower_expr(cx, target)?;
            let args = collect_args(cx, arguments);
            let ret = cx.temp();
            cx.emit(Op::Call {
                target: ret,
                func,
                args,
            });
            Some(ret)
        }

        _ => None,
    }
}

fn lower_binary(cx: &mut LowerCtx, node: &AstNode) -> Option<Slot> {
    let (opk, left, right) = match &node.kind {
        AstType::BinaryOp { op, left, right } => (op, left.as_ref(), right.as_ref()),
        _ => return None,
    };
    let l = lower_expr(cx, left)?;
    let r = lower_expr(cx, right)?;
    let out = cx.temp();
    match opk {
        BinaryOperator::Add => cx.emit(Op::Add {
            lhs: l,
            rhs: r,
            target: out,
        }),
        BinaryOperator::Sub => cx.emit(Op::Sub {
            lhs: l,
            rhs: r,
            target: out,
        }),
        BinaryOperator::Mul => cx.emit(Op::Mul {
            lhs: l,
            rhs: r,
            target: out,
        }),
        BinaryOperator::Div => cx.emit(Op::Div {
            lhs: l,
            rhs: r,
            target: out,
        }),
        BinaryOperator::Eq => cx.emit(Op::Eq {
            lhs: l,
            rhs: r,
            target: out,
        }),
        BinaryOperator::Ne => cx.emit(Op::Ne {
            lhs: l,
            rhs: r,
            target: out,
        }),
        BinaryOperator::Lt => cx.emit(Op::Lt {
            lhs: l,
            rhs: r,
            target: out,
        }),
        BinaryOperator::Le => cx.emit(Op::Le {
            lhs: l,
            rhs: r,
            target: out,
        }),
        BinaryOperator::Gt => cx.emit(Op::Gt {
            lhs: l,
            rhs: r,
            target: out,
        }),
        BinaryOperator::Ge => cx.emit(Op::Ge {
            lhs: l,
            rhs: r,
            target: out,
        }),
    }
    Some(out)
}
