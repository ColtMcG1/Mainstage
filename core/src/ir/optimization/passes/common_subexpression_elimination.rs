use std::collections::{HashMap, HashSet};
use crate::ir::{op::Op, slot::Slot};
use crate::ir::optimization::pass::Pass;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ExprKey {
    BinOp { kind: &'static str, lhs: Slot, rhs: Slot },
    UnOp  { kind: &'static str, src: Slot },
    Len   { src: Slot },
}

fn is_pure_op(op: &Op) -> Option<ExprKey> {
    match op {
        Op::Add { lhs, rhs, target: _ } => Some(ExprKey::BinOp { kind: "Add", lhs: *lhs, rhs: *rhs }),
        Op::Sub { lhs, rhs, target: _ } => Some(ExprKey::BinOp { kind: "Sub", lhs: *lhs, rhs: *rhs }),
        Op::Mul { lhs, rhs, target: _ } => Some(ExprKey::BinOp { kind: "Mul", lhs: *lhs, rhs: *rhs }),
        Op::Div { lhs, rhs, target: _ } => Some(ExprKey::BinOp { kind: "Div", lhs: *lhs, rhs: *rhs }),
        Op::Eq  { lhs, rhs, target: _ } => Some(ExprKey::BinOp { kind: "Eq",  lhs: *lhs, rhs: *rhs }),
        Op::Ne  { lhs, rhs, target: _ } => Some(ExprKey::BinOp { kind: "Ne",  lhs: *lhs, rhs: *rhs }),
        Op::Lt  { lhs, rhs, target: _ } => Some(ExprKey::BinOp { kind: "Lt",  lhs: *lhs, rhs: *rhs }),
        Op::Le  { lhs, rhs, target: _ } => Some(ExprKey::BinOp { kind: "Le",  lhs: *lhs, rhs: *rhs }),
        Op::Gt  { lhs, rhs, target: _ } => Some(ExprKey::BinOp { kind: "Gt",  lhs: *lhs, rhs: *rhs }),
        Op::Ge  { lhs, rhs, target: _ } => Some(ExprKey::BinOp { kind: "Ge",  lhs: *lhs, rhs: *rhs }),
        Op::Not { source, target: _ }   => Some(ExprKey::UnOp  { kind: "Not", src: *source }),
        Op::Length { target: _, array }  => Some(ExprKey::Len  { src: *array }),
        _ => None,
    }
}

pub struct CommonSubexpressionElimination;
impl CommonSubexpressionElimination { pub fn new() -> Self { Self } }

impl Pass for CommonSubexpressionElimination {
    fn name(&self) -> &'static str { "cse" }

    fn run(&mut self, ops: &mut Vec<Op>) -> bool {
        let mut changed = false;

        // aliases for slots produced as duplicates -> canonical slot
        let mut alias: HashMap<Slot, Slot> = HashMap::new();
        // expression -> canonical slot (for non-const expressions)
        let mut expr_map: HashMap<ExprKey, Slot> = HashMap::new();
        // keep const_map across blocks so identical LoadConst in different blocks canonicalize
        let mut const_map: HashMap<String, Slot> = HashMap::new();

        // NEW: collect slots that are used as array indices (be conservative)
        let mut index_slots: HashSet<Slot> = HashSet::new();
        for op in ops.iter() {
            match op {
                Op::IGet { index, .. } => { index_slots.insert(*index); }
                Op::ISet { index, .. } => { index_slots.insert(*index); }
                _ => {}
            }
        }

        // indices of ops that are duplicate pure producers and can be removed
        let mut remove_indices: Vec<usize> = Vec::new();

        // helper to canonicalize a slot through aliases (plain function to avoid borrow conflicts)
        fn resolve_slot(alias: &HashMap<Slot, Slot>, mut s: Slot) -> Slot {
            loop {
                if let Some(&a) = alias.get(&s) {
                    if a == s { break; }
                    s = a;
                    continue;
                }
                break;
            }
            s
        }

        // conservative: break at hard control-flow boundaries and on calls/side-effects
        let is_boundary = |op: &Op| {
            matches!(op, Op::Label { .. } | Op::Jump { .. } | Op::BrFalse { .. } |
                         Op::Return { .. } | Op::Halt | Op::Call { .. } |
                         Op::ISet { .. } | Op::MGet { .. } | Op::NewArray { .. })
        };

        let mut i = 0usize;
        while i < ops.len() {
            // reset maps at block start
            expr_map.clear();

            // process until boundary (inclusive)
            while i < ops.len() {
                // replace operands using alias map before any processing
                match &mut ops[i] {
                    Op::Add { lhs, rhs, .. } | Op::Sub { lhs, rhs, .. } |
                    Op::Mul { lhs, rhs, .. } | Op::Div { lhs, rhs, .. } |
                    Op::Eq  { lhs, rhs, .. } | Op::Ne  { lhs, rhs, .. } |
                    Op::Lt  { lhs, rhs, .. } | Op::Le  { lhs, rhs, .. } |
                    Op::Gt  { lhs, rhs, .. } | Op::Ge  { lhs, rhs, .. } => {
                        *lhs = resolve_slot(&alias, *lhs);
                        *rhs = resolve_slot(&alias, *rhs);
                    }
                    Op::Not { source, .. } => {
                        *source = resolve_slot(&alias, *source);
                    }
                    Op::Length { array, .. } => {
                        *array = resolve_slot(&alias, *array);
                    }
                    Op::IGet { source, index, .. } => {
                        *source = resolve_slot(&alias, *source);
                        *index  = resolve_slot(&alias, *index);
                    }
                    // update ISet operands as well (array, index, value)
                    Op::ISet { target, index, value } => {
                        *target = resolve_slot(&alias, *target);
                        *index  = resolve_slot(&alias, *index);
                        *value  = resolve_slot(&alias, *value);
                    }
                    Op::Call { args, .. } => {
                        for a in args.iter_mut() { *a = resolve_slot(&alias, *a); }
                    }
                    Op::StoreLocal { source, .. } => { *source = resolve_slot(&alias, *source); }
                    _ => {}
                }

                // Special-case LoadConst early so constants populate const_map even when
                // is_pure_op() returns None. This is why const_map was empty in your trace.
                if let Op::LoadConst { target, value } = &ops[i] {
                    // If this target is used as an array index elsewhere, do not alias/remove it.
                    if index_slots.contains(target) {
                        // keep it as-is; still ensure const_map has an entry so other duplicates can canonicalize
                        let k = format!("{:?}", value);
                        const_map.entry(k).or_insert(*target);
                        i += 1;
                        continue;
                    }

                    let k = format!("{:?}", value);
                    if let Some(&existing) = const_map.get(&k) {
                        // alias this LoadConst's target to existing canonical slot
                        alias.insert(*target, existing);
                        // schedule this duplicate producer for removal unless it's used as an index
                        remove_indices.push(i);
                        changed = true;
                    } else {
                        const_map.insert(k, *target);
                    }
                    // move to next op without double-processing this LoadConst
                    i += 1;
                    // treat LoadConst as not a block boundary here (behavior preserved)
                    continue;
                }
 
                // then try to detect duplicates (only for pure ops)
                if let Some(key) = is_pure_op(&ops[i]) {
                    // compute the canonical key with resolved slots
                    let canon_key = match key {
                        ExprKey::BinOp { kind, lhs, rhs } => ExprKey::BinOp { kind, lhs: resolve_slot(&alias, lhs), rhs: resolve_slot(&alias, rhs) },
                        ExprKey::UnOp { kind, src }       => ExprKey::UnOp  { kind, src: resolve_slot(&alias, src) },
                        ExprKey::Len { src }              => ExprKey::Len { src: resolve_slot(&alias, src) },
                    };

                    // determine target if this is a producer op (extract target slot)
                    let target_opt = match &ops[i] {
                        Op::Add { target, .. } => Some(*target),
                        Op::Sub { target, .. } => Some(*target),
                        Op::Mul { target, .. } => Some(*target),
                        Op::Div { target, .. } => Some(*target),
                        Op::Eq  { target, .. } => Some(*target),
                        Op::Ne  { target, .. } => Some(*target),
                        Op::Lt  { target, .. } => Some(*target),
                        Op::Le  { target, .. } => Some(*target),
                        Op::Gt  { target, .. } => Some(*target),
                        Op::Ge  { target, .. } => Some(*target),
                        Op::Not { target, .. } => Some(*target),
                        Op::Length { target, .. } => Some(*target),
                        _ => None
                    };

                    if let Some(tgt) = target_opt {
                        // If producer target is used as an index elsewhere, skip alias/removal
                        if index_slots.contains(&tgt) {
                            expr_map.insert(canon_key, tgt);
                        } else if let Some(&existing) = expr_map.get(&canon_key) {
                            // duplicate found: record alias from this target -> existing canonical slot
                            alias.insert(tgt, existing);
                            // schedule this duplicate producer for removal
                            remove_indices.push(i);
                            changed = true;
                        } else {
                            expr_map.insert(canon_key, tgt);
                        }
                    }
                }

                // move to next op; stop if this op is a boundary
                let stop = is_boundary(&ops[i]);
                i += 1;
                if stop { break; }
            }
        }

        // Remove duplicate pure producers that we recorded. Remove in descending order to keep indices valid.
        if !remove_indices.is_empty() {
            remove_indices.sort_unstable();
            remove_indices.dedup();
            for &idx in remove_indices.iter().rev() {
                // safe: idx < ops.len() because we recorded indices during the scan
                ops.remove(idx);
            }
            // continue with rewrite step using the updated ops vector
        }

        // Second pass: rewrite operands using final alias map so subsequent passes see canonical slots.
        let resolve_fn = |s: Slot| resolve_slot(&alias, s);

        for op in ops.iter_mut() {
            match op {
                Op::Add { lhs, rhs, .. } | Op::Sub { lhs, rhs, .. } |
                Op::Mul { lhs, rhs, .. } | Op::Div { lhs, rhs, .. } |
                Op::Eq  { lhs, rhs, .. } | Op::Ne  { lhs, rhs, .. } |
                Op::Lt  { lhs, rhs, .. } | Op::Le  { lhs, rhs, .. } |
                Op::Gt  { lhs, rhs, .. } | Op::Ge  { lhs, rhs, .. } => {
                    *lhs = resolve_fn(*lhs);
                    *rhs = resolve_fn(*rhs);
                }
                Op::Not { source, .. } => { *source = resolve_fn(*source); }
                Op::Length { array, .. } => { *array = resolve_fn(*array); }
                Op::IGet { source, index, .. } => { *source = resolve_fn(*source); *index = resolve_fn(*index); }
                Op::ISet { target, index, value } => {
                    *target = resolve_fn(*target);
                    *index  = resolve_fn(*index);
                    *value  = resolve_fn(*value);
                }
                Op::Call { args, .. } => { for a in args.iter_mut() { *a = resolve_fn(*a); } }
                Op::StoreLocal { source, .. } => { *source = resolve_fn(*source); }
                Op::Say { message } => {
                     let old = *message;
                     let new = resolve_fn(old);
                     if old != new {
                         *message = new;
                     }
                 }
                Op::Return { value } => {
                    if let Some(s) = value {
                        let old = *s;
                        let new = resolve_fn(old);
                        if old != new {
                            *value = Some(new);
                        }
                    }
                }
                _ => {}
            }
        }
        
        changed
    }
}