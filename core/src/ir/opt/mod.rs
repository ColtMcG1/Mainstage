//! IR optimizer placeholder module
// Future optimization passes should live here (constant-folding, dce, etc.)

// Keep the module present so `pub mod opt;` compiles while we migrate.

use crate::ir::lower::IrModule;
use crate::ir::op::IROp;
use crate::ir::value::Value;
use std::collections::HashMap;

/// Run optimization passes on the lowered IR in-place.
pub fn optimize(ir: &mut IrModule) {
    // Constant-fold simple arithmetic/comparison ops
    constant_fold(ir);

    // Remove no-op jumps/branches and reindex numeric targets and stage label indices
    remove_noop_jumps_and_reindex(ir);
}

fn constant_fold(ir: &mut IrModule) {
    use super::value::Value::*;

    let mut const_map: HashMap<usize, Value> = HashMap::new();
    let mut new_ops: Vec<IROp> = Vec::with_capacity(ir.ops.len());

    for op in ir.ops.drain(..) {
        match &op {
            IROp::LConst { dest, value } => {
                const_map.insert(*dest, value.clone());
                new_ops.push(op);
            }
            IROp::Add { dest, src1, src2 }
            | IROp::Sub { dest, src1, src2 }
            | IROp::Mul { dest, src1, src2 }
            | IROp::Div { dest, src1, src2 }
            | IROp::Mod { dest, src1, src2 }
            | IROp::Eq { dest, src1, src2 }
            | IROp::Neq { dest, src1, src2 }
            | IROp::Lt { dest, src1, src2 }
            | IROp::Lte { dest, src1, src2 }
            | IROp::Gt { dest, src1, src2 }
            | IROp::Gte { dest, src1, src2 }
            | IROp::And { dest, src1, src2 }
            | IROp::Or { dest, src1, src2 } => {
                let maybe_v1 = const_map.get(src1).cloned();
                let maybe_v2 = const_map.get(src2).cloned();
                if let (Some(v1), Some(v2)) = (maybe_v1, maybe_v2) {
                    if let Some(res) = compute_binop(&op, &v1, &v2) {
                        // fold into LConst
                        let d = *dest;
                        const_map.insert(d, res.clone());
                        new_ops.push(IROp::LConst {
                            dest: d,
                            value: res,
                        });
                        continue;
                    }
                }
                // not foldable: destination no longer constant
                const_map.remove(dest);
                new_ops.push(op);
            }
            IROp::Not { dest, src } => {
                if let Some(v) = const_map.get(src).cloned() {
                    match v {
                        Bool(b) => {
                            let d = *dest;
                            const_map.insert(d, Bool(!b));
                            new_ops.push(IROp::LConst {
                                dest: d,
                                value: Bool(!b),
                            });
                            continue;
                        }
                        _ => {}
                    }
                }
                const_map.remove(dest);
                new_ops.push(op);
            }
            // ops that write to a destination register should invalidate that register
            IROp::Inc { .. }
            | IROp::Dec { .. }
            | IROp::LLocal { .. }
            | IROp::Call { .. }
            | IROp::CallLabel { .. }
            | IROp::CLoad { .. } => {
                // dest may be updated by non-constant operation
                // remove any existing constant binding
                // Note: pattern matching above ensures Add/Sub handled earlier
                let d = match &op {
                    IROp::Inc { dest } => *dest,
                    IROp::Dec { dest } => *dest,
                    IROp::LLocal { dest, .. } => *dest,
                    IROp::Call { dest, .. } => *dest,
                    IROp::CallLabel { dest, .. } => *dest,
                    IROp::CLoad { dest, .. } => *dest,
                    _ => unreachable!(),
                };
                const_map.remove(&d);
                new_ops.push(op);
            }
            _ => {
                // most ops either don't create constants or are control flow; conservatively
                // invalidate any dest register we can detect by pattern matching common forms
                new_ops.push(op);
            }
        }
    }

    ir.ops = new_ops;
}

fn compute_binop(op: &IROp, v1: &Value, v2: &Value) -> Option<Value> {
    use super::value::Value::*;

    match (v1, v2) {
        (Int(a), Int(b)) => match op {
            IROp::Add { .. } => Some(Int(a + b)),
            IROp::Sub { .. } => Some(Int(a - b)),
            IROp::Mul { .. } => Some(Int(a * b)),
            IROp::Div { .. } => {
                if *b != 0 {
                    Some(Int(a / b))
                } else {
                    None
                }
            }
            IROp::Mod { .. } => {
                if *b != 0 {
                    Some(Int(a % b))
                } else {
                    None
                }
            }
            IROp::Eq { .. } => Some(Bool(a == b)),
            IROp::Neq { .. } => Some(Bool(a != b)),
            IROp::Lt { .. } => Some(Bool(a < b)),
            IROp::Lte { .. } => Some(Bool(a <= b)),
            IROp::Gt { .. } => Some(Bool(a > b)),
            IROp::Gte { .. } => Some(Bool(a >= b)),
            _ => None,
        },
        (Float(a), Float(b)) => match op {
            IROp::Add { .. } => Some(Float(a + b)),
            IROp::Sub { .. } => Some(Float(a - b)),
            IROp::Mul { .. } => Some(Float(a * b)),
            IROp::Div { .. } => {
                if *b != 0.0 {
                    Some(Float(a / b))
                } else {
                    None
                }
            }
            IROp::Eq { .. } => Some(Bool(a == b)),
            IROp::Neq { .. } => Some(Bool(a != b)),
            IROp::Lt { .. } => Some(Bool(a < b)),
            IROp::Lte { .. } => Some(Bool(a <= b)),
            IROp::Gt { .. } => Some(Bool(a > b)),
            IROp::Gte { .. } => Some(Bool(a >= b)),
            _ => None,
        },
        (Bool(a), Bool(b)) => match op {
            IROp::And { .. } => Some(Bool(*a && *b)),
            IROp::Or { .. } => Some(Bool(*a || *b)),
            IROp::Eq { .. } => Some(Bool(a == b)),
            IROp::Neq { .. } => Some(Bool(a != b)),
            _ => None,
        },
        _ => None,
    }
}

fn remove_noop_jumps_and_reindex(ir: &mut IrModule) {
    // Remove jumps/branches that target the next instruction (no-op)
    // and reindex all numeric targets to account for removed ops.
    let mut keep_flags: Vec<bool> = Vec::with_capacity(ir.ops.len());
    for i in 0..ir.ops.len() {
        match &ir.ops[i] {
            IROp::Jump { target } if *target == i + 1 => keep_flags.push(false),
            IROp::BrTrue { target, .. } if *target == i + 1 => keep_flags.push(false),
            IROp::BrFalse { target, .. } if *target == i + 1 => keep_flags.push(false),
            _ => keep_flags.push(true),
        }
    }

    // Build mapping old_index -> new_index
    let mut mapping: HashMap<usize, usize> = HashMap::new();
    let mut new_ops: Vec<IROp> = Vec::with_capacity(ir.ops.len());
    for (old_idx, op) in ir.ops.drain(..).enumerate() {
        if keep_flags[old_idx] {
            let new_idx = new_ops.len();
            mapping.insert(old_idx, new_idx);
            new_ops.push(op);
        }
    }

    // Now update numeric targets within new_ops
    for op in new_ops.iter_mut() {
        match op {
            IROp::Jump { target } => {
                if let Some(&n) = mapping.get(target) {
                    *target = n;
                }
            }
            IROp::BrTrue { target, .. } => {
                if let Some(&n) = mapping.get(target) {
                    *target = n;
                }
            }
            IROp::BrFalse { target, .. } => {
                if let Some(&n) = mapping.get(target) {
                    *target = n;
                }
            }
            IROp::CallLabel { label_index, .. } => {
                if let Some(&n) = mapping.get(label_index) {
                    *label_index = n;
                }
            }
            _ => {}
        }
    }

    // Update stage label indices
    for (_name, idx) in ir.get_stage_labels().iter_mut() {
        if let Some(&n) = mapping.get(idx) {
            *idx = n;
        }
    }

    ir.ops = new_ops;
}
