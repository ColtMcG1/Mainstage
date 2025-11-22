use std::collections::{HashMap, HashSet};
use crate::ir::op::Op;
use crate::ir::slot::Slot;
use crate::ir::value::OpValue;
use crate::ir::optimization::pass::Pass;

pub struct DeadCodeElim;
impl DeadCodeElim { pub fn new() -> Self { Self } }

impl Pass for DeadCodeElim {
    fn name(&self) -> &'static str { "dce" }

    fn run(&mut self, ops: &mut Vec<Op>) -> bool {
        let mut changed_any = false;

        // 1) Detect called scopes by tracking string LoadConst -> Call relationships.
        let mut const_map: HashMap<Slot, OpValue> = HashMap::new();
        let mut called_scopes: HashSet<String> = HashSet::new();

        for op in ops.iter() {
            match op {
                Op::LoadConst { target, value } => {
                    const_map.insert(*target, value.clone());
                }
                _ => {
                    // if this op defines a slot, conservatively forget any const mapping for that slot
                    if let Some(def) = op.defines_slot() {
                        const_map.remove(&def);
                    }
                }
            }

            if let Op::Call { func, .. } = op {
                if let Some(OpValue::Str(name)) = const_map.get(func) {
                    called_scopes.insert(name.clone());
                }
            }
        }

        // 2) Remove unused scope regions (labels "scope.<name>" where <name> not in called_scopes).
        let mut removed_scope = false;
        let mut filtered: Vec<Op> = Vec::with_capacity(ops.len());
        let mut i = 0usize;
        while i < ops.len() {
            if let Op::Label { name } = &ops[i] {
                if let Some(rest) = name.strip_prefix("scope.") {
                    if !called_scopes.contains(rest) {
                        // skip this scope region until next "scope." label or end
                        removed_scope = true;
                        let mut j = i + 1;
                        while j < ops.len() {
                            if let Op::Label { name: n2 } = &ops[j] {
                                if n2.starts_with("scope.") { break; }
                            }
                            j += 1;
                        }
                        i = j;
                        continue;
                    }
                }
            }
            filtered.push(ops[i].clone());
            i += 1;
        }

        if removed_scope {
            *ops = filtered;
            changed_any = true;
        }

        // 3) Run original DCE logic on the (possibly filtered) ops:
        // initial use counts (how many times each slot is read)
        let mut uses: HashMap<Slot, usize> = HashMap::new();
        for op in ops.iter() {
            op.each_used_slot(|s| { *uses.entry(s).or_insert(0) += 1; });
        }

        // which locals are ever read (LoadLocal)
        let mut local_reads: HashSet<Slot> = HashSet::new();
        for op in ops.iter() {
            if let Op::LoadLocal { local, .. } = op {
                local_reads.insert(*local);
            }
        }

        // decide removals in reverse so consumers are processed before producers
        let mut keep = vec![true; ops.len()];
        let mut changed = false;

        for i in (0..ops.len()).rev() {
            let op = &ops[i];

            // If this is a StoreLocal into a local that is never read -> remove it.
            if let Op::StoreLocal { source, target } = op {
                if !local_reads.contains(target) {
                    keep[i] = false;
                    changed = true;
                    // decrement use count for the source so its producer can become removable
                    if let Some(c) = uses.get_mut(source) {
                        if *c > 0 { *c -= 1; }
                    }
                    continue;
                }
            }

            // Remove pure producers with zero uses.
            if op.is_pure() {
                if let Some(def) = op.defines_slot() {
                    if uses.get(&def).copied().unwrap_or(0) == 0 {
                        // mark removable and decrement uses of any operands it used
                        keep[i] = false;
                        changed = true;
                        op.each_used_slot(|s| {
                            if let Some(c) = uses.get_mut(&s) {
                                if *c > 0 { *c -= 1; }
                            }
                        });
                        continue;
                    }
                }
            }
        }

        if changed {
            // rebuild ops keeping only marked ones (preserve original order)
            let mut out = Vec::with_capacity(ops.len());
            for (i, op) in ops.iter().enumerate() {
                if keep[i] { out.push(op.clone()); }
            }
            *ops = out;
            changed_any = true;
        }

        changed_any
    }
}