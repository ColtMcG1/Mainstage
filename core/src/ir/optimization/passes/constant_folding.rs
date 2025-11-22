use std::collections::{HashMap, HashSet};
use crate::ir::{op::Op, slot::Slot, value::OpValue};
use crate::ir::value::OpValue as V;
use crate::ir::optimization::pass::Pass;

pub struct ConstantFolding;
impl ConstantFolding { pub fn new() -> Self { Self } }

impl Pass for ConstantFolding {
    fn name(&self) -> &'static str { "constant_folding" }

    fn run(&mut self, ops: &mut Vec<Op>) -> bool {
        let mut changed = false;

        // conservative: detect mutated locals so we don't propagate across them
        let mut mutated_locals: HashSet<Slot> = HashSet::new();
        for op in ops.iter() {
            match op {
                Op::StoreLocal { target, .. } => { mutated_locals.insert(*target); }
                Op::Inc { target } | Op::Dec { target } => { mutated_locals.insert(*target); }
                Op::ISet { target, .. } => { mutated_locals.insert(*target); }
                _ => {}
            }
        }

        let mut consts: HashMap<Slot, OpValue> = HashMap::new();

        for i in 0..ops.len() {
            // clear facts at hard control-flow boundaries to stay conservative
            match ops[i] {
                Op::Label { .. } | Op::Jump { .. } | Op::Return { .. } | Op::Halt => {
                    consts.clear();
                }
                _ => {}
            }

            // We'll compute an optional replacement const for this op without mutating `consts`
            let mut folded_const: Option<(Slot, OpValue)> = None;
            // and a list of slots to invalidate if we can't fold the op's target
            let mut invalidate_target: Option<Slot> = None;

            match &ops[i] {
                Op::LoadConst { target, value } => {
                    // remember const
                    folded_const = Some((*target, value.clone()));
                }

                Op::LoadLocal { target, local } => {
                    // only propagate local constants if that local isn't mutated
                    if !mutated_locals.contains(local) {
                        if let Some(v) = consts.get(local) {
                            folded_const = Some((*target, v.clone()));
                        } else {
                            invalidate_target = Some(*target);
                        }
                    } else {
                        invalidate_target = Some(*target);
                    }
                }

                // Globals are identified by name (String). we don't track them in the Slot->OpValue map.
                // Be conservative: invalidate the destination slot instead of looking up by name.
                Op::LoadGlobal { target, .. } => {
                    invalidate_target = Some(*target);
                }

                Op::StoreLocal { source, target } => {
                    // if source is a tracked temp constant, we can propagate it to local (unless mutated)
                    if !mutated_locals.contains(target) {
                        if let Some(v) = consts.get(source) {
                            folded_const = Some((*target, v.clone()));
                        } else {
                            // storing a non-const invalidates local const
                            consts.remove(target);
                        }
                    } else {
                        consts.remove(target);
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
                    let l = consts.get(lhs);
                    let r = consts.get(rhs);

                    let res = match &ops[i] {
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

                    match res {
                        Some(v) => { folded_const = Some((*target, v)); }
                        None => { invalidate_target = Some(*target); }
                    }
                }

                Op::Length { target, array } => {
                    match consts.get(array) {
                        Some(V::Array(a)) => folded_const = Some((*target, V::Int(a.len() as i64))),
                        Some(V::Str(s))   => folded_const = Some((*target, V::Int(s.len() as i64))),
                        _ => invalidate_target = Some(*target),
                    }
                }

                Op::IGet { target, source, index } => {
                    match (consts.get(source), consts.get(index)) {
                        (Some(V::Array(arr)), Some(V::Int(i))) if *i >= 0 => {
                            let ui = *i as usize;
                            if ui < arr.len() {
                                folded_const = Some((*target, arr[ui].clone()));
                            } else {
                                invalidate_target = Some(*target);
                            }
                        }
                        _ => invalidate_target = Some(*target),
                    }
                }

                Op::Not { source, target } => {
                    match consts.get(source) {
                        Some(V::Bool(b)) => folded_const = Some((*target, V::Bool(!b))),
                        _ => invalidate_target = Some(*target),
                    }
                }

                Op::Call { target, .. } => {
                    invalidate_target = Some(*target);
                }

                Op::NewArray { target, .. }
                | Op::ISet { target, .. }
                | Op::MGet { target, .. } => {
                    invalidate_target = Some(*target);
                }

                Op::Inc { target } | Op::Dec { target } => {
                    // mutating operations invalidate any const knowledge of the slot
                    consts.remove(target);
                }

                _ => {}
            }

            // Apply deferred invalidation first
            if let Some(slot) = invalidate_target {
                consts.remove(&slot);
            }

            // Apply folded constant (if any) — and replace op in the instruction stream
            if let Some((slot, val)) = folded_const {
                // if this instruction is not already a LoadConst for the same slot, replace it
                match &ops[i] {
                    Op::LoadConst { target, value: _ } if *target == slot => {
                        // already the right LoadConst; still ensure map updated
                        consts.insert(slot, val);
                    }
                    _ => {
                        ops[i] = Op::LoadConst { target: slot, value: val.clone() };
                        consts.insert(slot, val);
                        changed = true;
                    }
                }
            }
        }

        changed
    }
}

// helpers

fn as_int(v: &OpValue) -> Option<i64> {
    match v { V::Int(i) => Some(*i), _ => None }
}
fn as_float(v: &OpValue) -> Option<f64> {
    match v { V::Float(f) => Some(*f), _ => None }
}
fn both_int(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<(i64,i64)> {
    Some((as_int(l?)?, as_int(r?)?))
}
fn both_float(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<(f64,f64)> {
    Some((as_float(l?)?, as_float(r?)?))
}

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
        (V::Int(a),   V::Int(b))   => Some(V::Bool(a == b)),
        (V::Float(a), V::Float(b)) => Some(V::Bool(a == b)),
        (V::Bool(a),  V::Bool(b))  => Some(V::Bool(a == b)),
        (V::Str(a),   V::Str(b))   => Some(V::Bool(a == b)),
        (V::Null,     V::Null)     => Some(V::Bool(true)),
        _ => None,
    }
}
fn fold_ne(l: Option<&OpValue>, r: Option<&OpValue>) -> Option<OpValue> {
    fold_eq(l,r).map(|v| if let V::Bool(b) = v { V::Bool(!b) } else { v })
}

#[derive(Clone, Copy)]
enum Cmp { Lt, Le, Gt, Ge }

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
