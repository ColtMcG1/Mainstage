use crate::ir::optimization::pass::Pass;
use crate::ir::{op::Op, slot::Slot, value::OpValue};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
enum LVal {
    Unknown,
    Const(OpValue),
    Overdefined,
}

pub struct SparseConditionalConstantProp;
impl SparseConditionalConstantProp {
    pub fn new() -> Self {
        Self
    }
}

impl Pass for SparseConditionalConstantProp {
    fn name(&self) -> &'static str {
        "sccp"
    }

    fn run(&mut self, ops: &mut Vec<Op>) -> bool {
        use LVal::*;
        let mut changed = false;

        // map slot -> lattice value
        let mut lattice: HashMap<Slot, LVal> = HashMap::new();
        // set of mutated locals (conservative)
        let mut mutated_locals: HashSet<Slot> = HashSet::new();

        // mark mutated locals first (StoreLocal, Inc/Dec, ISet, etc.)
        for op in ops.iter() {
            match op {
                Op::StoreLocal { target, .. }
                | Op::ISet { target, .. }
                | Op::Inc { target }
                | Op::Dec { target } => {
                    mutated_locals.insert(*target);
                }
                _ => {}
            }
        }

        // helper: read lattice value
        let get =
            |m: &HashMap<Slot, LVal>, s: Slot| -> LVal { m.get(&s).cloned().unwrap_or(Unknown) };

        // helper: write lattice value, returns true if changed
        let set_val = |m: &mut HashMap<Slot, LVal>, s: Slot, v: LVal| -> bool {
            match (m.get(&s).cloned(), v) {
                (None, Unknown) => false,
                (None, new) => {
                    m.insert(s, new);
                    true
                }
                (Some(Unknown), Unknown) => false,
                (Some(Unknown), new) => {
                    m.insert(s, new);
                    true
                }
                (Some(Const(old)), Const(ref newv)) if old == *newv => false,
                (Some(Const(_)), Const(newv)) => {
                    m.insert(s, Const(newv));
                    true
                } // different constant -> update (rare)
                (_, Overdefined) => {
                    if !matches!(m.get(&s), Some(Overdefined)) {
                        m.insert(s, Overdefined);
                        true
                    } else {
                        false
                    }
                }
                (Some(Const(_)), Unknown) => false,
                (Some(Overdefined), _) => false,
            }
        };

        // pure ops that can produce constants
        let is_pure_and_eval = |op: &Op, m: &HashMap<Slot, LVal>| -> Option<OpValue> {
            match op {
                Op::LoadConst { value, .. } => Some(value.clone()),
                Op::Add { lhs, rhs, .. }
                | Op::Sub { lhs, rhs, .. }
                | Op::Mul { lhs, rhs, .. }
                | Op::Div { lhs, rhs, .. } => {
                    let lv = m.get(lhs).cloned().unwrap_or(Unknown);
                    let rv = m.get(rhs).cloned().unwrap_or(Unknown);
                    match (lv, rv) {
                        (LVal::Const(lv), LVal::Const(rv)) => {
                            // simple numeric ops; keep only int/float combinations handled by existing helpers
                            match (lv, rv) {
                                (OpValue::Int(a), OpValue::Int(b)) => {
                                    return Some(match op {
                                        Op::Add { .. } => OpValue::Int(a + b),
                                        Op::Sub { .. } => OpValue::Int(a - b),
                                        Op::Mul { .. } => OpValue::Int(a * b),
                                        Op::Div { .. } => OpValue::Int(a / b),
                                        _ => unreachable!(),
                                    });
                                }
                                (OpValue::Float(a), OpValue::Float(b)) => {
                                    return Some(match op {
                                        Op::Add { .. } => OpValue::Float(a + b),
                                        Op::Sub { .. } => OpValue::Float(a - b),
                                        Op::Mul { .. } => OpValue::Float(a * b),
                                        Op::Div { .. } => OpValue::Float(a / b),
                                        _ => unreachable!(),
                                    });
                                }
                                // mixed int/float -> promote to float
                                (OpValue::Int(a), OpValue::Float(b)) => {
                                    let a = a as f64;
                                    return Some(match op {
                                        Op::Add { .. } => OpValue::Float(a + b),
                                        Op::Sub { .. } => OpValue::Float(a - b),
                                        Op::Mul { .. } => OpValue::Float(a * b),
                                        Op::Div { .. } => OpValue::Float(a / b),
                                        _ => unreachable!(),
                                    });
                                }
                                (OpValue::Float(a), OpValue::Int(b)) => {
                                    let b = b as f64;
                                    return Some(match op {
                                        Op::Add { .. } => OpValue::Float(a + b),
                                        Op::Sub { .. } => OpValue::Float(a - b),
                                        Op::Mul { .. } => OpValue::Float(a * b),
                                        Op::Div { .. } => OpValue::Float(a / b),
                                        _ => unreachable!(),
                                    });
                                }
                                _ => None,
                            }
                        }
                        _ => None,
                    }
                }
                Op::Eq { lhs, rhs, .. }
                | Op::Ne { lhs, rhs, .. }
                | Op::Lt { lhs, rhs, .. }
                | Op::Le { lhs, rhs, .. }
                | Op::Gt { lhs, rhs, .. }
                | Op::Ge { lhs, rhs, .. } => {
                    let lv = m.get(lhs).cloned().unwrap_or(Unknown);
                    let rv = m.get(rhs).cloned().unwrap_or(Unknown);
                    match (lv, rv) {
                        (LVal::Const(lv), LVal::Const(rv)) => {
                            // equality/relational on ints/strings supported
                            return match (lv, rv) {
                                (OpValue::Int(a), OpValue::Int(b)) => {
                                    Some(OpValue::Bool(match op {
                                        Op::Eq { .. } => a == b,
                                        Op::Ne { .. } => a != b,
                                        Op::Lt { .. } => a < b,
                                        Op::Le { .. } => a <= b,
                                        Op::Gt { .. } => a > b,
                                        Op::Ge { .. } => a >= b,
                                        _ => unreachable!(),
                                    }))
                                }
                                (OpValue::Str(a), OpValue::Str(b)) => {
                                    Some(OpValue::Bool(match op {
                                        Op::Eq { .. } => a == b,
                                        Op::Ne { .. } => a != b,
                                        Op::Lt { .. } => a < b,
                                        Op::Le { .. } => a <= b,
                                        Op::Gt { .. } => a > b,
                                        Op::Ge { .. } => a >= b,
                                        _ => unreachable!(),
                                    }))
                                }
                                _ => None,
                            };
                        }
                        _ => None,
                    }
                }
                Op::Not { source, .. } => match m.get(source).cloned().unwrap_or(Unknown) {
                    LVal::Const(OpValue::Bool(b)) => Some(OpValue::Bool(!b)),
                    _ => None,
                },
                Op::Length { array, .. } => match m.get(array).cloned().unwrap_or(Unknown) {
                    LVal::Const(OpValue::Array(a)) => Some(OpValue::Int(a.len() as i64)),
                    LVal::Const(OpValue::Str(s)) => Some(OpValue::Int(s.len() as i64)),
                    _ => None,
                },
                _ => None,
            }
        };

        // Worklist iter until fixed point — iterate over ops repeatedly until no change.
        let mut work = true;
        let mut iter = 0usize;
        while work && iter < 1000 {
            iter += 1;
            work = false;

            for op in ops.iter() {
                match op {
                    Op::LoadConst { target, value } => {
                        if set_val(&mut lattice, *target, LVal::Const(value.clone())) {
                            work = true;
                        }
                    }

                    Op::LoadLocal { target, local } => {
                        // conservative: if local not mutated and we have a const for the local slot, propagate it.
                        if !mutated_locals.contains(local) {
                            let val = lattice.get(local).cloned().unwrap_or(Unknown);
                            match val {
                                LVal::Const(v) => {
                                    if set_val(&mut lattice, *target, LVal::Const(v)) {
                                        work = true;
                                    }
                                }
                                LVal::Overdefined => {
                                    if set_val(&mut lattice, *target, Overdefined) {
                                        work = true;
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            if set_val(&mut lattice, *target, Overdefined) {
                                work = true;
                            }
                        }
                    }

                    // store local makes that local overdefined (conservative)
                    Op::StoreLocal { source, target } => {
                        let src_val = lattice.get(source).cloned().unwrap_or(Unknown);
                        match src_val {
                            LVal::Const(_) => {
                                // track value on the local slot only if it was always that const (we conservatively mark mutated locals earlier)
                                // but we still mark local as overdefined for safety
                                mutated_locals.insert(*target);
                                if set_val(&mut lattice, *target, Overdefined) {
                                    work = true;
                                }
                            }
                            LVal::Overdefined => {
                                mutated_locals.insert(*target);
                                if set_val(&mut lattice, *target, Overdefined) {
                                    work = true;
                                }
                            }
                            _ => {
                                mutated_locals.insert(*target);
                                if set_val(&mut lattice, *target, Overdefined) {
                                    work = true;
                                }
                            }
                        }
                    }

                    // arithmetic/comparison/unary/length: try to evaluate to const
                    op if matches!(
                        op,
                        Op::Add { .. }
                            | Op::Sub { .. }
                            | Op::Mul { .. }
                            | Op::Div { .. }
                            | Op::Eq { .. }
                            | Op::Ne { .. }
                            | Op::Lt { .. }
                            | Op::Le { .. }
                            | Op::Gt { .. }
                            | Op::Ge { .. }
                            | Op::Not { .. }
                            | Op::Length { .. }
                    ) =>
                    {
                        // get the target slot
                        let target = match op {
                            Op::Add { target, .. } => *target,
                            Op::Sub { target, .. } => *target,
                            Op::Mul { target, .. } => *target,
                            Op::Div { target, .. } => *target,
                            Op::Eq { target, .. } => *target,
                            Op::Ne { target, .. } => *target,
                            Op::Lt { target, .. } => *target,
                            Op::Le { target, .. } => *target,
                            Op::Gt { target, .. } => *target,
                            Op::Ge { target, .. } => *target,
                            Op::Not { target, .. } => *target,
                            Op::Length { target, .. } => *target,
                            _ => unreachable!(),
                        };

                        if let Some(val) = is_pure_and_eval(op, &lattice) {
                            if set_val(&mut lattice, target, LVal::Const(val)) {
                                work = true;
                            }
                        } else {
                            // if operands are known Overdefined -> mark target Overdefined
                            // find dependent slots and if any Overdefined => target Overdefined
                            let dep_over = match op {
                                Op::Add { lhs, rhs, .. }
                                | Op::Sub { lhs, rhs, .. }
                                | Op::Mul { lhs, rhs, .. }
                                | Op::Div { lhs, rhs, .. }
                                | Op::Eq { lhs, rhs, .. }
                                | Op::Ne { lhs, rhs, .. }
                                | Op::Lt { lhs, rhs, .. }
                                | Op::Le { lhs, rhs, .. }
                                | Op::Gt { lhs, rhs, .. }
                                | Op::Ge { lhs, rhs, .. } => {
                                    matches!(get(&lattice, *lhs), Overdefined)
                                        || matches!(get(&lattice, *rhs), Overdefined)
                                }
                                Op::Not { source, .. } => {
                                    matches!(get(&lattice, *source), Overdefined)
                                }
                                Op::Length { array, .. } => {
                                    matches!(get(&lattice, *array), Overdefined)
                                }
                                _ => false,
                            };
                            if dep_over {
                                let tgt = match op {
                                    Op::Add { .. }
                                    | Op::Sub { .. }
                                    | Op::Mul { .. }
                                    | Op::Div { .. }
                                    | Op::Eq { .. }
                                    | Op::Ne { .. }
                                    | Op::Lt { .. }
                                    | Op::Le { .. }
                                    | Op::Gt { .. }
                                    | Op::Ge { .. }
                                    | Op::Not { .. }
                                    | Op::Length { .. } => match op {
                                        Op::Add { target, .. } => *target,
                                        Op::Sub { target, .. } => *target,
                                        Op::Mul { target, .. } => *target,
                                        Op::Div { target, .. } => *target,
                                        Op::Eq { target, .. } => *target,
                                        Op::Ne { target, .. } => *target,
                                        Op::Lt { target, .. } => *target,
                                        Op::Le { target, .. } => *target,
                                        Op::Gt { target, .. } => *target,
                                        Op::Ge { target, .. } => *target,
                                        Op::Not { target, .. } => *target,
                                        Op::Length { target, .. } => *target,
                                        _ => unreachable!(),
                                    },
                                    _ => unreachable!(),
                                };
                                if set_val(&mut lattice, tgt, Overdefined) {
                                    work = true;
                                }
                            }
                        }
                    }

                    // other ops with targets are considered overdefined
                    Op::Call { .. }
                    | Op::NewArray { .. }
                    | Op::ISet { .. }
                    | Op::MGet { .. } => {
                        // targets are overdefined
                        if let Some(t) = match op {
                            Op::Call { target, .. } => Some(*target),
                            Op::NewArray { target, .. } => Some(*target),
                            Op::ISet { target, .. } => Some(*target),
                            Op::MGet { target, .. } => Some(*target),
                            _ => None,
                        } {
                            if set_val(&mut lattice, t, Overdefined) {
                                work = true;
                            }
                        }
                    }
                    
                    _ => {}
                }
            }
        } // end worklist

        // Replace proven-constant producers with LoadConst to help later passes.
        for op in ops.iter_mut() {
            match op {
                Op::Add { target, .. }
                | Op::Sub { target, .. }
                | Op::Mul { target, .. }
                | Op::Div { target, .. }
                | Op::Eq { target, .. }
                | Op::Ne { target, .. }
                | Op::Lt { target, .. }
                | Op::Le { target, .. }
                | Op::Gt { target, .. }
                | Op::Ge { target, .. }
                | Op::Not { target, .. }
                | Op::Length { target, .. } => {
                    if let Some(LVal::Const(v)) = lattice.get(target).cloned() {
                        // replace op with LoadConst
                        *op = Op::LoadConst {
                            target: *target,
                            value: v,
                        };
                        changed = true;
                    }
                }
                Op::LoadLocal { target, local } => {
                    if let Some(LVal::Const(v)) = lattice.get(local).cloned() {
                        *op = Op::LoadConst {
                            target: *target,
                            value: v,
                        };
                        changed = true;
                    }
                }
                _ => {}
            }
        }

        changed
    }
}
