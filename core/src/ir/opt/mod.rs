//! IR optimizer module: constant-folding and simple interprocedural substitution
use crate::ir::lower::IrModule;
use crate::ir::op::IROp;
use crate::ir::value::Value;
use std::collections::HashMap;

/// Run optimization passes on the lowered IR in-place.
pub fn optimize(ir: &mut IrModule) {
    // First: try to propagate local constants across call boundaries using lowering metadata.
    // Doing this before constant-folding preserves op indices recorded during lowering
    // which some substitution algorithms rely on.
    interproc_substitute(ir);

    // Then constant-fold simple arithmetic/comparison ops
    constant_fold(ir);

    // Finally remove trivial jumps and reindex numeric targets and stage label indices
    remove_noop_jumps_and_reindex(ir);
}

fn constant_fold(ir: &mut IrModule) {
    use super::value::Value::*;

    let mut const_map: HashMap<usize, Value> = HashMap::new();
    // Track locals that are known constants: local_index -> Value
    let mut local_const_map: HashMap<usize, Value> = HashMap::new();
    let mut new_ops: Vec<IROp> = Vec::with_capacity(ir.ops.len());

    for op in ir.ops.drain(..) {
        match &op {
            IROp::LConst { dest, value } => {
                const_map.insert(*dest, value.clone());
                new_ops.push(op);
            }
            IROp::SLocal { src, local_index } => {
                // If the source register is a known constant, mark the local as constant.
                if let Some(v) = const_map.get(src).cloned() {
                    local_const_map.insert(*local_index, v);
                } else {
                    local_const_map.remove(local_index);
                }
                new_ops.push(op);
            }
            IROp::LLocal { dest, local_index } => {
                // If the local has a known constant value, fold into LConst.
                if let Some(v) = local_const_map.get(local_index).cloned() {
                    const_map.insert(*dest, v.clone());
                    new_ops.push(IROp::LConst { dest: *dest, value: v });
                } else {
                    const_map.remove(dest);
                    new_ops.push(op);
                }
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
                        let d = *dest;
                        const_map.insert(d, res.clone());
                        new_ops.push(IROp::LConst { dest: d, value: res });
                        continue;
                    }
                }
                const_map.remove(dest);
                new_ops.push(op);
            }
            IROp::Not { dest, src } => {
                if let Some(v) = const_map.get(src).cloned() {
                    match v {
                        Bool(b) => {
                            let d = *dest;
                            const_map.insert(d, Bool(!b));
                            new_ops.push(IROp::LConst { dest: d, value: Bool(!b) });
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
            | IROp::Call { .. }
            | IROp::CallLabel { .. }
            | IROp::CLoad { .. } => {
                let d = match &op {
                    IROp::Inc { dest } => *dest,
                    IROp::Dec { dest } => *dest,
                    IROp::Call { dest, .. } => *dest,
                    IROp::CallLabel { dest, .. } => *dest,
                    IROp::CLoad { dest, .. } => *dest,
                    _ => unreachable!(),
                };
                const_map.remove(&d);
                new_ops.push(op);
            }
            _ => {
                new_ops.push(op);
            }
        }
    }

    ir.ops = new_ops;
}

// Helper: compute simple binary op results at constant-fold time
fn compute_binop(op: &IROp, v1: &Value, v2: &Value) -> Option<Value> {
    use super::value::Value::*;
    match op {
        IROp::Add { .. } => match (v1, v2) {
            (Int(a), Int(b)) => Some(Int(a + b)),
            (Float(a), Float(b)) => Some(Float(a + b)),
            (Int(a), Float(b)) => Some(Float((*a as f64) + b)),
            (Float(a), Int(b)) => Some(Float(a + (*b as f64))),
            (Str(a), Str(b)) => Some(Str(format!("{}{}", a, b))),
            _ => None,
        },
        IROp::Sub { .. } => match (v1, v2) {
            (Int(a), Int(b)) => Some(Int(a - b)),
            (Float(a), Float(b)) => Some(Float(a - b)),
            (Int(a), Float(b)) => Some(Float((*a as f64) - b)),
            (Float(a), Int(b)) => Some(Float(a - (*b as f64))),
            _ => None,
        },
        IROp::Mul { .. } => match (v1, v2) {
            (Int(a), Int(b)) => Some(Int(a * b)),
            (Float(a), Float(b)) => Some(Float(a * b)),
            (Int(a), Float(b)) => Some(Float((*a as f64) * b)),
            (Float(a), Int(b)) => Some(Float(a * (*b as f64))),
            _ => None,
        },
        IROp::Div { .. } => match (v1, v2) {
            (Int(a), Int(b)) => if *b == 0 { None } else { Some(Int(a / b)) },
            (Float(a), Float(b)) => if *b == 0.0 { None } else { Some(Float(a / b)) },
            (Int(a), Float(b)) => if *b == 0.0 { None } else { Some(Float((*a as f64) / b)) },
            (Float(a), Int(b)) => if *b == 0 { None } else { Some(Float(a / (*b as f64))) },
            _ => None,
        },
        IROp::Mod { .. } => match (v1, v2) {
            (Int(a), Int(b)) => if *b == 0 { None } else { Some(Int(a % b)) },
            _ => None,
        },
        IROp::Eq { .. } => Some(Value::Bool(v1 == v2)),
        IROp::Neq { .. } => Some(Value::Bool(v1 != v2)),
        IROp::Lt { .. } => match (v1, v2) {
            (Int(a), Int(b)) => Some(Value::Bool(a < b)),
            (Float(a), Float(b)) => Some(Value::Bool(a < b)),
            (Int(a), Float(b)) => Some(Value::Bool((*a as f64) < *b)),
            (Float(a), Int(b)) => Some(Value::Bool(*a < (*b as f64))),
            (Str(a), Str(b)) => Some(Value::Bool(a < b)),
            _ => None,
        },
        IROp::Lte { .. } => match (v1, v2) {
            (Int(a), Int(b)) => Some(Value::Bool(a <= b)),
            (Float(a), Float(b)) => Some(Value::Bool(a <= b)),
            (Int(a), Float(b)) => Some(Value::Bool((*a as f64) <= *b)),
            (Float(a), Int(b)) => Some(Value::Bool(*a <= (*b as f64))),
            (Str(a), Str(b)) => Some(Value::Bool(a <= b)),
            _ => None,
        },
        IROp::Gt { .. } => match (v1, v2) {
            (Int(a), Int(b)) => Some(Value::Bool(a > b)),
            (Float(a), Float(b)) => Some(Value::Bool(a > b)),
            (Int(a), Float(b)) => Some(Value::Bool((*a as f64) > *b)),
            (Float(a), Int(b)) => Some(Value::Bool(*a > (*b as f64))),
            (Str(a), Str(b)) => Some(Value::Bool(a > b)),
            _ => None,
        },
        IROp::Gte { .. } => match (v1, v2) {
            (Int(a), Int(b)) => Some(Value::Bool(a >= b)),
            (Float(a), Float(b)) => Some(Value::Bool(a >= b)),
            (Int(a), Float(b)) => Some(Value::Bool((*a as f64) >= *b)),
            (Float(a), Int(b)) => Some(Value::Bool(*a >= (*b as f64))),
            (Str(a), Str(b)) => Some(Value::Bool(a >= b)),
            _ => None,
        },
        IROp::And { .. } => match (v1, v2) {
            (Value::Bool(a), Value::Bool(b)) => Some(Value::Bool(*a && *b)),
            _ => None,
        },
        IROp::Or { .. } => match (v1, v2) {
            (Value::Bool(a), Value::Bool(b)) => Some(Value::Bool(*a || *b)),
            _ => None,
        },
        _ => None,
    }
}

fn interproc_substitute(ir: &mut IrModule) {
    // Use lowering metadata to specialize callee bodies when caller locals are compile-time constants.
    let slocal_names = ir.get_op_slocal_name();
    let stage_labels = ir.get_stage_labels();

    // Build reverse map: label_index -> stage_name
    let mut label_to_stage: HashMap<usize, String> = HashMap::new();
    for (name, idx) in stage_labels.iter() {
        label_to_stage.insert(*idx, name.clone());
    }

    let mut i = 0usize;
    while i < ir.ops.len() {
        let op_i = ir.ops[i].clone();
        if let IROp::CallLabel { label_index, args: call_args_current, .. } = op_i {
            if let Some(stage_name) = label_to_stage.get(&label_index) {
                if let Some(param_names) = ir.get_stage_param_names(stage_name) {
                    if let Some(param_local_indices) = ir.get_stage_param_local_indices(stage_name) {
                        // synthesize missing args from caller SLocal variables when possible
                        let mut args = call_args_current.clone();
                        let mut inserted = 0usize;
                        if args.len() < param_names.len() {
                            let call_pos = i;
                            for pi in args.len()..param_names.len() {
                                let pname = &param_names[pi];
                                let mut found_slocal: Option<usize> = None;
                                for j in (0..call_pos).rev() {
                                    if let Some(nm) = slocal_names.get(&j) {
                                        if nm == pname {
                                            if let IROp::SLocal { .. } = &ir.ops[j] {
                                                found_slocal = Some(j);
                                                break;
                                            }
                                        }
                                    }
                                }
                                if let Some(sidx) = found_slocal {
                                    if let IROp::SLocal { src: _s_src, local_index: s_local } = ir.ops[sidx].clone() {
                                        // choose fresh dest register
                                        let mut max_reg: usize = 0;
                                        for op in ir.ops.iter() {
                                            match op {
                                                IROp::LConst { dest, .. }
                                                | IROp::LLocal { dest, .. }
                                                | IROp::Add { dest, .. }
                                                | IROp::Sub { dest, .. }
                                                | IROp::Mul { dest, .. }
                                                | IROp::Div { dest, .. }
                                                | IROp::Mod { dest, .. }
                                                | IROp::Eq { dest, .. }
                                                | IROp::Neq { dest, .. }
                                                | IROp::Lt { dest, .. }
                                                | IROp::Lte { dest, .. }
                                                | IROp::Gt { dest, .. }
                                                | IROp::Gte { dest, .. }
                                                | IROp::And { dest, .. }
                                                | IROp::Or { dest, .. }
                                                | IROp::Not { dest, .. }
                                                | IROp::Inc { dest }
                                                | IROp::Dec { dest }
                                                | IROp::Call { dest, .. }
                                                | IROp::CallLabel { dest, .. }
                                                | IROp::CLoad { dest, .. }
                                                | IROp::AllocClosure { dest }
                                                => if *dest > max_reg { max_reg = *dest; }
                                                _ => {}
                                            }
                                        }
                                        let new_reg = max_reg + 1;
                                        ir.ops.insert(call_pos, IROp::LLocal { dest: new_reg, local_index: s_local });
                                        inserted += 1;
                                        args.push(new_reg);
                                    }
                                }
                            }
                            if inserted > 0 {
                                let updated_call_pos = i + inserted;
                                match &mut ir.ops[updated_call_pos] {
                                    IROp::CallLabel { args: aargs, .. } => { *aargs = args.clone(); }
                                    _ => {}
                                }
                                i += inserted;
                            }
                        }

                        // Replace callee LLocal reads with LConst when caller provided a literal
                        for (pi, pname) in param_names.iter().enumerate() {
                            if pi >= param_local_indices.len() { break; }
                            let param_local_idx = param_local_indices[pi];

                            let mut found_slocal: Option<usize> = None;
                            for j in (0..i).rev() {
                                if let Some(nm) = slocal_names.get(&j) {
                                    if nm == pname {
                                        if let IROp::SLocal { .. } = &ir.ops[j] {
                                            found_slocal = Some(j);
                                            break;
                                        }
                                    }
                                }
                            }
                            if found_slocal.is_none() { continue; }
                            let sidx = found_slocal.unwrap();
                            let src_reg = if let IROp::SLocal { src, .. } = &ir.ops[sidx] { *src } else { continue; };

                            let mut literal_val: Option<Value> = None;
                            for k in (0..sidx).rev() {
                                if let IROp::LConst { dest, value } = &ir.ops[k] {
                                    if *dest == src_reg { literal_val = Some(value.clone()); break; }
                                }
                            }
                            if literal_val.is_none() { continue; }
                            let val = literal_val.unwrap();

                            let start = label_index;
                            let mut end = ir.ops.len();
                            for t in (start + 1)..ir.ops.len() {
                                if let IROp::Label { .. } = &ir.ops[t] { end = t; break; }
                            }

                            for t in (start + 1)..end {
                                let op_clone = ir.ops[t].clone();
                                if let IROp::LLocal { dest, local_index } = op_clone {
                                    if local_index == param_local_idx {
                                        ir.ops[t] = IROp::LConst { dest, value: val.clone() };
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        i += 1;
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
                if let Some(&n) = mapping.get(target) { *target = n; }
            }
            IROp::BrTrue { target, .. } => {
                if let Some(&n) = mapping.get(target) { *target = n; }
            }
            IROp::BrFalse { target, .. } => {
                if let Some(&n) = mapping.get(target) { *target = n; }
            }
            IROp::CallLabel { label_index, .. } => {
                if let Some(&n) = mapping.get(label_index) { *label_index = n; }
            }
            _ => {}
        }
    }

    // Update stage label indices
    for (_name, idx) in ir.get_stage_labels().iter_mut() {
        if let Some(&n) = mapping.get(idx) { *idx = n; }
    }

    ir.ops = new_ops;
}
