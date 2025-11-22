use std::collections::{HashMap, HashSet};
use crate::ir::{op::Op, slot::Slot, value::OpValue};
use crate::ir::optimization::pass::Pass;

/// Constant Propagation
/// - Propagates known constants stored in locals/temps to later LoadLocal/LoadGlobal.
/// - Conservative: respects mutated locals and clears at labels/jumps/returns.
pub struct ConstantPropagation;
impl ConstantPropagation { pub fn new() -> Self { Self } }

impl Pass for ConstantPropagation {
    fn name(&self) -> &'static str { "const_prop" }

    fn run(&mut self, ops: &mut Vec<Op>) -> bool {
        // detect mutated locals (StoreLocal / Inc/Dec)
        let mut mutated: HashSet<Slot> = HashSet::new();
        for op in ops.iter() {
            match op {
                Op::StoreLocal { target, .. } => { mutated.insert(*target); }
                Op::Inc { target } | Op::Dec { target } => { mutated.insert(*target); }
                _ => {}
            }
        }

        let mut changed = false;
        let mut reg_consts: HashMap<Slot, OpValue> = HashMap::new();
        let mut local_consts: HashMap<Slot, OpValue> = HashMap::new();

        let len = ops.len();
        let mut i = 0;
        while i < len {
            // Take a clone of the current op for matching, to avoid borrowing ops immutably and mutably at the same time
            let op_clone = ops[i].clone();

            // clear maps at boundaries to avoid cross-block unsound propagation
            match op_clone {
                Op::Label { .. } | Op::Jump { .. } | Op::Return { .. } | Op::Halt => {
                    reg_consts.clear();
                    local_consts.clear();
                }
                _ => {}
            }

            match op_clone {
                Op::LoadConst { target, ref value } => {
                    reg_consts.insert(target, value.clone());
                }

                Op::LoadLocal { target, local } => {
                    // Only propagate if local not mutated
                    if !mutated.contains(&local) {
                        if let Some(val) = local_consts.get(&local) {
                            // replace load with load const
                            ops[i] = Op::LoadConst { target, value: val.clone() };
                            reg_consts.insert(target, val.clone());
                            changed = true;
                        } else {
                            // not known
                            reg_consts.remove(&target);
                        }
                    } else {
                        reg_consts.remove(&target);
                    }
                }

                Op::LoadGlobal { target, .. } => {
                    // Cannot propagate global constants with local_consts (different key types)
                    reg_consts.remove(&target);
                }

                Op::StoreLocal { source, target } => {
                    // If source is a known constant (temp), store it as local const (unless mutated)
                    if !mutated.contains(&target) {
                        if let Some(v) = reg_consts.get(&source) {
                            local_consts.insert(target, v.clone());
                        } else {
                            local_consts.remove(&target);
                        }
                    } else {
                        local_consts.remove(&target);
                    }
                }

                Op::Call { target, .. } => { reg_consts.remove(&target); }

                Op::Inc { target } | Op::Dec { target } => {
                    reg_consts.remove(&target);
                    local_consts.remove(&target);
                }

                Op::NewArray { target, .. } | Op::ISet { target, .. } | Op::MGet { target, .. } => {
                    reg_consts.remove(&target);
                }

                // for binary/pure ops we just invalidate the target; folding handled by constant folding pass
                Op::Add { target, .. } | Op::Sub { target, .. } | Op::Mul { target, .. } |
                Op::Div { target, .. } | Op::Eq { target, .. } | Op::Ne { target, .. } |
                Op::Lt { target, .. } | Op::Le { target, .. } | Op::Gt { target, .. } |
                Op::Ge { target, .. } | Op::Length { target, .. } | Op::IGet { target, .. } => {
                    // we don't compute here; just invalidate any previous knowledge about target
                    reg_consts.remove(&target);
                }

                _ => {}
            }
            i += 1;
        }

        changed
    }
}