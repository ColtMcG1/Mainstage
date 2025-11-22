use std::collections::{HashMap, HashSet};
use crate::ir::{op::Op, slot::Slot, value::OpValue};
use crate::ir::value::OpValue as V;
use crate::ir::optimization::pass::Pass;

pub struct ConstantFolding;
impl ConstantFolding { pub fn new() -> Self { Self } }

impl Pass for ConstantFolding {
    fn name(&self) -> &'static str { "constant_folding" }

    fn run(&mut self, ops: &mut Vec<Op>) -> bool {
        // Pre-scan: find mutated locals (multiple assignments or Inc/Dec)
        let mut mutated: HashSet<Slot> = HashSet::new();
        let mut assigns: HashMap<Slot, usize> = HashMap::new();
        for op in ops.iter() {
            match op {
                Op::StoreLocal { target, .. } => {
                    let c = assigns.entry(*target).or_insert(0);
                    *c += 1;
                    if *c > 1 { mutated.insert(*target); }
                }
                Op::Inc { target } | Op::Dec { target } => {
                    mutated.insert(*target);
                }
                _ => {}
            }
        }

        let mut changed = false;
        let mut reg_consts: HashMap<Slot, OpValue> = HashMap::new();
        let mut local_consts: HashMap<Slot, OpValue> = HashMap::new();

        for i in 0..ops.len() {
            // treat labels & jumps as boundaries for locals too (avoid loop folding)
            match ops[i] {
                Op::Label { .. } | Op::Jump { .. } => {
                    reg_consts.clear();
                    local_consts.clear();
                }
                Op::Return { .. } | Op::Halt => {
                    reg_consts.clear();
                    local_consts.clear();
                }
                _ => {}
            }

            let mut replace_with: Option<(Slot, OpValue)> = None;
            let mut invalidate: Vec<Slot> = Vec::new();

            match &ops[i] {
                Op::LoadConst { target, value } => {
                    replace_with = Some((*target, value.clone()));
                }

                Op::LoadLocal { target, source } => {
                    if !mutated.contains(source) {
                        if let Some(v) = local_consts.get(source) {
                            replace_with = Some((*target, v.clone()));
                        }
                    } else {
                        invalidate.push(*target);
                    }
                }

                Op::StoreLocal { source, target } => {
                    if !mutated.contains(target) {
                        if let Some(v) = reg_consts.get(source) {
                            local_consts.insert(*target, v.clone());
                        } else {
                            local_consts.remove(target);
                        }
                    } else {
                        local_consts.remove(target);
                    }
                }

                Op::Add { lhs, rhs, target }
                | Op::Sub { lhs, rhs, target }
                | Op::Mul { lhs, rhs, target }
                | Op::Div { lhs, rhs, target }
                | Op::Eq  { lhs, rhs, target }
                | Op::Ne  { lhs, rhs, target }
                | Op::Lt  { lhs, rhs, target }
                | Op::Le  { lhs, rhs, target }
                | Op::Gt  { lhs, rhs, target }
                | Op::Ge  { lhs, rhs, target } => {
                    let l = reg_consts.get(lhs).or_else(|| local_consts.get(lhs));
                    let r = reg_consts.get(rhs).or_else(|| local_consts.get(rhs));
                    let folded = match &ops[i] {
                        Op::Add { .. } => fold_add(l, r),
                        Op::Sub { .. } => fold_sub(l, r),
                        Op::Mul { .. } => fold_mul(l, r),
                        Op::Div { .. } => fold_div(l, r),
                        Op::Eq  { .. } => fold_eq(l, r),
                        Op::Ne  { .. } => fold_ne(l, r),
                        Op::Lt  { .. } => fold_cmp(Cmp::Lt, l, r),
                        Op::Le  { .. } => fold_cmp(Cmp::Le, l, r),
                        Op::Gt  { .. } => fold_cmp(Cmp::Gt, l, r),
                        Op::Ge  { .. } => fold_cmp(Cmp::Ge, l, r),
                        _ => None,
                    };
                    if let Some(v) = folded {
                        replace_with = Some((*target, v));
                    } else {
                        invalidate.push(*target);
                    }
                }

                Op::Length { target, array } => {
                    if let Some(v) = reg_consts.get(array).or_else(|| local_consts.get(array)) {
                        match v {
                            V::Array(a) => replace_with = Some((*target, V::Int(a.len() as i64))),
                            V::Str(s) => replace_with = Some((*target, V::Int(s.len() as i64))),
                            _ => invalidate.push(*target),
                        }
                    } else {
                        invalidate.push(*target);
                    }
                }

                Op::IGet { target, source, index } => {
                    let src = reg_consts.get(source).or_else(|| local_consts.get(source));
                    let idx_v = reg_consts.get(index).or_else(|| local_consts.get(index));
                    if let (Some(V::Array(arr)), Some(V::Int(i))) = (src, idx_v) {
                        if *i >= 0 {
                            let ui = *i as usize;
                            if ui < arr.len() {
                                replace_with = Some((*target, arr[ui].clone()));
                            } else {
                                invalidate.push(*target);
                            }
                        } else {
                            invalidate.push(*target);
                        }
                    } else {
                        invalidate.push(*target);
                    }
                }

                Op::Inc { target } | Op::Dec { target } => {
                    reg_consts.remove(target);
                    local_consts.remove(target);
                }

                Op::Call { target, .. } => invalidate.push(*target),

                Op::NewArray { target, .. }
                | Op::ISet { target, .. }
                | Op::MGet { target, .. } => invalidate.push(*target),

                _ => {}
            }

            if let Some((slot, value)) = replace_with {
                match ops[i] {
                    Op::LoadConst { target, .. } if target == slot => {}
                    _ => {
                        ops[i] = Op::LoadConst { target: slot, value: value.clone() };
                        changed = true;
                    }
                }
                reg_consts.insert(slot, value);
            }
            for s in invalidate {
                reg_consts.remove(&s);
            }
        }
        changed
    }
}

// helpers unchanged
fn as_int(v: &OpValue) -> Option<i64> { match v { V::Int(i) => Some(*i), _ => None } }
fn as_float(v: &OpValue) -> Option<f64> { match v { V::Float(f) => Some(*f), _ => None } }
fn both_int(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<(i64,i64)> { Some((as_int(l?)?, as_int(r?)?)) }
fn both_float(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<(f64,f64)> { Some((as_float(l?)?, as_float(r?)?)) }
fn fold_add(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<OpValue> {
    if let Some((a,b)) = both_int(l,r) { return Some(V::Int(a + b)); }
    if let Some((a,b)) = both_float(l,r) { return Some(V::Float(a + b)); }
    if let (Some(V::Str(a)), Some(V::Str(b))) = (l,r) { return Some(V::Str(format!("{}{}", a,b))); }
    None
}
fn fold_sub(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<OpValue> {
    if let Some((a,b)) = both_int(l,r) { return Some(V::Int(a - b)); }
    if let Some((a,b)) = both_float(l,r) { return Some(V::Float(a - b)); }
    None
}
fn fold_mul(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<OpValue> {
    if let Some((a,b)) = both_int(l,r) { return Some(V::Int(a * b)); }
    if let Some((a,b)) = both_float(l,r) { return Some(V::Float(a * b)); }
    None
}
fn fold_div(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<OpValue> {
    if let Some((a,b)) = both_int(l,r) {
        if b == 0 { return None; }
        return Some(V::Int(a / b));
    }
    if let Some((a,b)) = both_float(l,r) {
        if b == 0.0 { return None; }
        return Some(V::Float(a / b));
    }
    None
}
fn fold_eq(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<OpValue> {
    match (l?, r?) {
        (V::Int(a), V::Int(b)) => Some(V::Bool(a == b)),
        (V::Float(a), V::Float(b)) => Some(V::Bool(a == b)),
        (V::Bool(a), V::Bool(b)) => Some(V::Bool(a == b)),
        (V::Str(a), V::Str(b)) => Some(V::Bool(a == b)),
        (V::Null, V::Null) => Some(V::Bool(true)),
        _ => None,
    }
}
fn fold_ne(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<OpValue> {
    fold_eq(l,r).map(|v| if let V::Bool(b) = v { V::Bool(!b) } else { v })
}
#[derive(Clone, Copy)] enum Cmp { Lt, Le, Gt, Ge }
fn fold_cmp(op: Cmp, l: Option<&OpValue>, r: Option<&OpValue>) -> Option<OpValue> {
    if let Some((a,b)) = both_int(l,r) {
        let res = match op { Cmp::Lt => a < b, Cmp::Le => a <= b, Cmp::Gt => a > b, Cmp::Ge => a >= b };
        return Some(V::Bool(res));
    }
    if let Some((a,b)) = both_float(l,r) {
        let res = match op { Cmp::Lt => a < b, Cmp::Le => a <= b, Cmp::Gt => a > b, Cmp::Ge => a >= b };
        return Some(V::Bool(res));
    }
    if let (Some(V::Str(a)), Some(V::Str(b))) = (l,r) {
        let res = match op { Cmp::Lt => a < b, Cmp::Le => a <= b, Cmp::Gt => a > b, Cmp::Ge => a >= b };
        return Some(V::Bool(res));
    }
    None
}
