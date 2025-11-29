use crate::ir::{ op::IROp };
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct IrModule {
    pub ops: Vec<IROp>,
    next_reg: usize,
    next_id: u32,
    functions: Vec<String>,
    objects: Vec<String>,
    labels: HashMap<String, usize>,
    unresolved_branches: Vec<(usize, String)>,
}

impl IrModule {
    pub fn new() -> Self {
        IrModule {
            ops: Vec::new(),
            next_reg: 0,
            next_id: 1,
            functions: Vec::new(),
            objects: Vec::new(),
            labels: HashMap::new(),
            unresolved_branches: Vec::new(),
        }
    }

    /// Allocate a fresh virtual register index for use by lowering.
    pub fn alloc_reg(&mut self) -> usize {
        let r = self.next_reg;
        self.next_reg = self.next_reg.wrapping_add(1);
        r
    }

    pub fn emit_op(&mut self, op: IROp) {
        // record the index where the op will be inserted
        let idx = self.ops.len();
        // push the op
        self.ops.push(op.clone());
        // Debug: log each emission so we can trace which lowering emitted which ops
        eprintln!("[ir-emit] idx={} op={}", idx, op);
        // if this op is a Label, record its position for later patching
        if let IROp::Label { name } = &op {
            self.labels.insert(name.clone(), idx);
        }
    }

    pub fn peek_op(&self) -> Option<&IROp> {
        self.ops.last()
    }

    pub fn pop_op(&mut self) -> Option<IROp> {
        self.ops.pop()
    }

    pub fn get_ops(&self) -> &Vec<IROp> {
        &self.ops
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Check whether any previously-emitted op wrote to the given register
    /// index. Used by finalization to avoid remapping operands that refer to
    /// module-level registers.
    pub fn reg_has_writer(&self, reg: usize) -> bool {
        for op in self.ops.iter() {
            match op {
                IROp::LConst { dest, .. } if *dest == reg => return true,
                IROp::ArrayNew { dest, .. } if *dest == reg => return true,
                IROp::ArrayGet { dest, .. } if *dest == reg => return true,
                IROp::Call { dest, .. } if *dest == reg => return true,
                IROp::CallLabel { dest, .. } if *dest == reg => return true,
                IROp::Add { dest, .. } if *dest == reg => return true,
                IROp::Lt { dest, .. } if *dest == reg => return true,
                IROp::LLocal { dest, .. } if *dest == reg => return true,
                IROp::Ret { src } if *src == reg => return true,
                _ => {}
            }
        }
        false
    }

    /// Record an unresolved branch that should be patched to a label later.
    /// `op_index` is the index of the branch op in `ops`, and `label_name` is
    /// the `IROp::Label` name that will be emitted later at the final target
    /// position.
    pub fn record_unresolved_branch(&mut self, op_index: usize, label_name: String) {
        self.unresolved_branches.push((op_index, label_name));
    }

    /// Patch any unresolved branches recorded earlier. This resolves branch
    /// placeholders (which were emitted with a dummy target) to the final
    /// op indices where the corresponding `Label` ops were emitted.
    pub fn patch_unresolved_branches(&mut self) {
        eprintln!("[ir] patch_unresolved_branches: labels={} unresolved={}", self.labels.len(), self.unresolved_branches.len());
        // Diagnostic scan: find BrFalse ops whose condition is produced by
        // an LConst Null (or otherwise note the last writer). This helps
        // track down missing comparison emissions during lowering.
        for (i, op) in self.ops.iter().enumerate() {
            if let IROp::BrFalse { cond, target: _ } = op {
                // search backwards for last op that wrote into `cond`
                if let Some(j) = (0..i).rev().find(|&k| {
                    match &self.ops[k] {
                        IROp::LConst { dest, .. }
                        | IROp::Add { dest, .. }
                        | IROp::Lt { dest, .. }
                        | IROp::Lte { dest, .. }
                        | IROp::Gt { dest, .. }
                        | IROp::Gte { dest, .. }
                        | IROp::Eq { dest, .. }
                        | IROp::Neq { dest, .. }
                        | IROp::Not { dest, .. }
                        | IROp::ArrayGet { dest, .. } => {
                            *dest == *cond
                        }
                        _ => false,
                    }
                }) {
                    eprintln!("[ir-diagnose] BrFalse at op={} cond=r{} last_writer_idx={} op={}", i, cond, j, self.ops[j]);
                } else {
                    eprintln!("[ir-diagnose] BrFalse at op={} cond=r{} has no prior writer", i, cond);
                }
            }
        }

        // Additional diagnostic: for any ArrayGet, report last writer for
        // both the `array` and `index` registers so we can see why reads
        // may return Null at runtime.
        for (i, op) in self.ops.iter().enumerate() {
            if let IROp::ArrayGet { dest: _, array, index } = op {
                let find_writer = |reg: &usize| -> Option<usize> {
                    (0..i).rev().find(|&k| match &self.ops[k] {
                        IROp::LConst { dest, .. } if dest == reg => true,
                        IROp::ArrayNew { dest, .. } if dest == reg => true,
                        IROp::ArrayGet { dest, .. } if dest == reg => true,
                        IROp::Call { dest, .. } if dest == reg => true,
                        IROp::CallLabel { dest, .. } if dest == reg => true,
                        IROp::Add { dest, .. } if dest == reg => true,
                        IROp::Lt { dest, .. } if dest == reg => true,
                        IROp::LLocal { dest, .. } if dest == reg => true,
                        _ => false,
                    })
                };
                let wa = find_writer(array);
                let wi = find_writer(index);
                eprintln!("[ir-diagnose] ArrayGet at op={} array=r{} last_writer_array={:?} index=r{} last_writer_index={:?}", i, array, wa, index, wi);
            }
        }
        for (op_index, label_name) in self.unresolved_branches.drain(..) {
            eprintln!("[ir] resolving branch at {} -> '{}'", op_index, label_name);
            if let Some(&target_idx) = self.labels.get(&label_name) {
                if op_index < self.ops.len() {
                    match &mut self.ops[op_index] {
                        IROp::BrFalse { cond: _, target } => { *target = target_idx; }
                        IROp::BrTrue { cond: _, target } => { *target = target_idx; }
                        IROp::Jump { target } => { *target = target_idx; }
                        other => {
                            eprintln!("[ir] attempted to patch non-branch op at {}: {}", op_index, other);
                        }
                    }
                } else {
                    eprintln!("[ir] unresolved branch op_index out of range: {}", op_index);
                }
            } else {
                eprintln!("[ir] unresolved branch: label '{}' not found", label_name);
            }
        }
        // Fallback: any remaining branch ops with target==0 likely point to
        // the next label emitted after them. Patch those automatically by
        // searching forward for a Label op.
        let mut patched_fallback = 0usize;
        for i in 0..self.ops.len() {
            // inspect op immutably first to avoid mutable/immutable borrow conflicts
            match &self.ops[i] {
                IROp::BrFalse { cond: _, target } if *target == 0 => {
                    // find next label immutably
                    if let Some(j) = (i+1..self.ops.len()).find(|&k| matches!(&self.ops[k], IROp::Label { .. })) {
                        // now mutate the op to set target
                        if let IROp::BrFalse { cond: _, target: tgt } = &mut self.ops[i] { *tgt = j; patched_fallback += 1; }
                    } else {
                        eprintln!("[ir] fallback patch: no label found after op {}", i);
                    }
                }
                IROp::BrTrue { cond: _, target } if *target == 0 => {
                    if let Some(j) = (i+1..self.ops.len()).find(|&k| matches!(&self.ops[k], IROp::Label { .. })) {
                        if let IROp::BrTrue { cond: _, target: tgt } = &mut self.ops[i] { *tgt = j; patched_fallback += 1; }
                    } else {
                        eprintln!("[ir] fallback patch: no label found after op {}", i);
                    }
                }
                IROp::Jump { target } if *target == 0 => {
                    if let Some(j) = (i+1..self.ops.len()).find(|&k| matches!(&self.ops[k], IROp::Label { .. })) {
                        if let IROp::Jump { target: tgt } = &mut self.ops[i] { *tgt = j; patched_fallback += 1; }
                    } else {
                        eprintln!("[ir] fallback patch: no label found after op {}", i);
                    }
                }
                _ => {}
            }
        }
        if patched_fallback > 0 {
            eprintln!("[ir] fallback patched {} branch(es)", patched_fallback);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Declare a function prototype in the module and return a function id.
    /// This is a thin registration API intended for lowering to reserve
    /// function identifiers before emitting bodies. The current implementation
    /// stores the name and returns a numeric id; expand this to store
    /// prototype metadata as needed.
    pub fn declare_function(&mut self, name: &str) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
            eprintln!("[ir-debug] declare_function -> id={} name='{}' next_function_id={}", id, name, self.next_id);
            self.functions.push(name.to_string());
        id
    }

    /// Declare an object (workspace/project) and return an object id.
    pub fn declare_object(&mut self, name: &str) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.objects.push(name.to_string());
        id
    }

    /// Optional helpers to inspect declared names (useful for tests/debugging).
    pub fn get_function_name(&self, id: u32) -> Option<&str> {
        let idx = (id - 1) as usize;
        self.functions.get(idx).map(|s| s.as_str())
    }

    pub fn get_object_name(&self, id: u32) -> Option<&str> {
        let idx = (id - 1) as usize;
        self.objects.get(idx).map(|s| s.as_str())
    }
}

impl std::fmt::Display for IrModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, op) in self.ops.iter().enumerate() {
            writeln!(f, "{:04}: {}", i, op)?;
        }
        Ok(())
    }
}