
use crate::ast::{AstNode, AstNodeKind, BinaryOperator, UnaryOperator};
use super::op::IROp;
use super::value::Value;

use std::collections::HashMap;

#[derive(Debug)]
pub struct IrModule {
    pub ops: Vec<IROp>,
    next_reg: usize,
    /// Per-function/local symbol -> local index mapping
    locals: Vec<HashMap<String, usize>>,
    /// Counter for generating unique internal labels
    label_counter: usize,
    /// Relocations for jumps/branches: (op_index, label_name)
    relocations: Vec<(usize, String)>,
        /// Map of stage name -> label op index (for direct call lowering)
        stage_labels: HashMap<String, usize>,
}

impl IrModule {
    pub fn new() -> Self {
        IrModule {
            ops: Vec::new(),
            next_reg: 0,
            locals: Vec::new(),
            label_counter: 0,
            relocations: Vec::new(),
            stage_labels: HashMap::new(),
        }
    }

    fn alloc_reg(&mut self) -> usize {
        let r = self.next_reg;
        self.next_reg += 1;
        r
    }

    fn current_locals_mut(&mut self) -> &mut HashMap<String, usize> {
        if self.locals.is_empty() {
            self.locals.push(HashMap::new());
        }
        self.locals.last_mut().unwrap()
    }

    pub fn lower_from_ast(&mut self, ast: &AstNode, entrypoint: &str) {
        // First, lower stage declarations so their labels exist for calls.
        if let AstNodeKind::Script { body } = &ast.kind {
            for n in body.iter() {
                if let AstNodeKind::Stage { .. } = &n.kind {
                    // When lowering a stage we only want to register its label and
                    // body; walk_node handles that.
                    self.walk_node(n);
                }
            }
        }

        // Then, find the workspace matching entrypoint and lower its body as
        // the script's execution entry. If not found, fall back to the first
        // workspace encountered.
        let mut fallback: Option<&AstNode> = None;
        if let AstNodeKind::Script { body } = &ast.kind {
            for n in body.iter() {
                if let AstNodeKind::Workspace { name, body } = &n.kind {
                    if fallback.is_none() {
                        fallback = Some(&n);
                    }
                    if name == entrypoint {
                        // Lower the workspace body as top-level code
                        self.walk_node(body);
                        // After lowering entrypoint body, resolve relocations and return
                        self.patch_relocations();
                        return;
                    }
                }
            }
        }

        // If entrypoint workspace wasn't found, lower the first workspace body if present
        if let Some(ws) = fallback {
            if let AstNodeKind::Workspace { body, .. } = &ws.kind {
                self.walk_node(body);
            }
        }

        // Resolve label relocations (jump targets)
        self.patch_relocations();
    }

    fn new_label(&mut self) -> String {
        let n = self.label_counter;
        self.label_counter += 1;
        format!("L_internal_{}", n)
    }

    fn emit_label(&mut self, name: String) {
        let pos = self.ops.len();
        self.ops.push(IROp::Label { name: name.clone() });
        self.stage_labels.insert(name, pos);
    }

    fn emit_jump_to(&mut self, label: String) {
        let pos = self.ops.len();
        self.ops.push(IROp::Jump { target: 0 });
        self.relocations.push((pos, label));
    }

    fn emit_brfalse_to(&mut self, cond: usize, label: String) {
        let pos = self.ops.len();
        self.ops.push(IROp::BrFalse { cond, target: 0 });
        self.relocations.push((pos, label));
    }

    fn emit_brtrue_to(&mut self, cond: usize, label: String) {
        let pos = self.ops.len();
        self.ops.push(IROp::BrTrue { cond, target: 0 });
        self.relocations.push((pos, label));
    }

    fn patch_relocations(&mut self) {
        // Build label -> index map
        let mut label_pos: HashMap<String, usize> = HashMap::new();
        for (i, op) in self.ops.iter().enumerate() {
            if let IROp::Label { name } = op {
                label_pos.insert(name.clone(), i);
            }
        }

        for (pos, label) in self.relocations.iter() {
            if let Some(&target_idx) = label_pos.get(label) {
                // Replace the op at pos with an updated one
                match &self.ops[*pos] {
                    IROp::Jump { .. } => {
                        self.ops[*pos] = IROp::Jump { target: target_idx };
                    }
                    IROp::BrFalse { cond, .. } => {
                        let c = *cond;
                        self.ops[*pos] = IROp::BrFalse { cond: c, target: target_idx };
                    }
                    IROp::BrTrue { cond, .. } => {
                        let c = *cond;
                        self.ops[*pos] = IROp::BrTrue { cond: c, target: target_idx };
                    }
                    _ => {}
                }
            }
        }
    }

    fn push_locals_scope(&mut self) {
        self.locals.push(HashMap::new());
    }
    fn pop_locals_scope(&mut self) {
        self.locals.pop();
    }

    fn walk_node(&mut self, node: &AstNode) -> Option<usize> {
        match &node.kind {
            AstNodeKind::Script { body } => {
                for n in body.iter() {
                    self.walk_node(n);
                }
                None
            }
            AstNodeKind::Stage { name, args, body } => {
                // Emit a label for the stage
                self.emit_label(name.clone());
                // Enter a new locals scope for this stage
                self.push_locals_scope();

                // If parameters present, register them as locals (in order)
                if let Some(params_node) = args {
                    if let AstNodeKind::Arguments { args: params } = &params_node.kind {
                        let locals = self.current_locals_mut();
                        for p in params.iter() {
                            if let AstNodeKind::Identifier { name: pname } = &p.kind {
                                let idx = locals.len();
                                locals.insert(pname.clone(), idx);
                            }
                        }
                    }
                }

                self.walk_node(body);
                // Ensure function returns by halting if none
                self.ops.push(IROp::Halt);
                self.pop_locals_scope();
                None
            }
            AstNodeKind::If { condition, body } => {
                let end_label = self.new_label();
                let cond_reg = self.walk_node(condition).unwrap_or_else(|| self.alloc_reg());
                self.emit_brfalse_to(cond_reg, end_label.clone());
                self.walk_node(body);
                self.emit_label(end_label);
                None
            }
            AstNodeKind::IfElse { condition, if_body, else_body } => {
                let else_label = self.new_label();
                let end_label = self.new_label();
                let cond_reg = self.walk_node(condition).unwrap_or_else(|| self.alloc_reg());
                self.emit_brfalse_to(cond_reg, else_label.clone());
                self.walk_node(if_body);
                self.emit_jump_to(end_label.clone());
                self.emit_label(else_label.clone());
                self.walk_node(else_body);
                self.emit_label(end_label);
                None
            }
            AstNodeKind::While { condition, body } => {
                let start = self.new_label();
                let end = self.new_label();
                self.emit_label(start.clone());
                let cond_reg = self.walk_node(condition).unwrap_or_else(|| self.alloc_reg());
                self.emit_brfalse_to(cond_reg, end.clone());
                self.walk_node(body);
                self.emit_jump_to(start.clone());
                self.emit_label(end);
                None
            }
            AstNodeKind::ForTo { initializer, limit, body } => {
                // Lower numeric for loops like: for i = start to limit { ... }
                // initializer should be an assignment to an identifier
                self.walk_node(initializer);
                // Attempt to detect the loop variable name from initializer
                let mut loop_var_idx: Option<usize> = None;
                if let AstNodeKind::Assignment { target, .. } = &initializer.kind {
                    if let AstNodeKind::Identifier { name } = &target.kind {
                        if let Some(idx) = self.current_locals_mut().get(name) {
                            loop_var_idx = Some(*idx);
                        }
                    }
                }

                let limit_reg = self.walk_node(limit).unwrap_or_else(|| self.alloc_reg());
                let start_lbl = self.new_label();
                let end_lbl = self.new_label();
                self.emit_label(start_lbl.clone());

                if let Some(idx) = loop_var_idx {
                    // load loop var into a register
                    let lv = self.alloc_reg();
                    self.ops.push(IROp::LLocal { dest: lv, local_index: idx });
                    let cmp = self.alloc_reg();
                    self.ops.push(IROp::Lt { dest: cmp, src1: lv, src2: limit_reg });
                    self.emit_brfalse_to(cmp, end_lbl.clone());
                    self.walk_node(body);
                    // increment local: lv = lv + 1
                    let one = self.alloc_reg();
                    self.ops.push(IROp::LConst { dest: one, value: Value::Int(1) });
                    let newv = self.alloc_reg();
                    self.ops.push(IROp::Add { dest: newv, src1: lv, src2: one });
                    self.ops.push(IROp::SLocal { src: newv, local_index: idx });
                    self.emit_jump_to(start_lbl.clone());
                }

                self.emit_label(end_lbl);
                None
            }
            AstNodeKind::ForIn { iterator, iterable, body } => {
                // If iterable is a literal list, unroll into sequential executions
                if let AstNodeKind::List { elements } = &iterable.kind {
                    // ensure iterator local exists
                    let locals = self.current_locals_mut();
                    let idx_local = if let Some(i) = locals.get(iterator) { *i } else { let ni = locals.len(); locals.insert(iterator.clone(), ni); ni };
                    for elem in elements.iter() {
                        // evaluate element into a register or constant and store into iterator local
                        let val_reg = if let AstNodeKind::Integer { value } = &elem.kind {
                            let r = self.alloc_reg();
                            self.ops.push(IROp::LConst { dest: r, value: Value::Int(*value) });
                            r
                        } else if let AstNodeKind::Float { value } = &elem.kind {
                            let r = self.alloc_reg();
                            self.ops.push(IROp::LConst { dest: r, value: Value::Float(*value) });
                            r
                        } else if let AstNodeKind::Bool { value } = &elem.kind {
                            let r = self.alloc_reg();
                            self.ops.push(IROp::LConst { dest: r, value: Value::Bool(*value) });
                            r
                        } else if let AstNodeKind::String { value } = &elem.kind {
                            let r = self.alloc_reg();
                            self.ops.push(IROp::LConst { dest: r, value: Value::Str(value.clone()) });
                            r
                        } else {
                            // fallback evaluate
                            self.walk_node(elem).unwrap_or_else(|| self.alloc_reg())
                        };
                        self.ops.push(IROp::SLocal { src: val_reg, local_index: idx_local });
                        self.walk_node(body);
                    }
                } else {
                    // General iterable lowering using runtime helpers `len(iterable)` and `index(iterable, idx)`.
                    // Evaluate iterable into a register
                    let iter_reg = self.walk_node(iterable).unwrap_or_else(|| self.alloc_reg());

                    // call len(iterable) -> len_reg
                    let len_func = self.alloc_reg();
                    self.ops.push(IROp::LConst { dest: len_func, value: Value::Str("len".to_string()) });
                    let len_dest = self.alloc_reg();
                    self.ops.push(IROp::Call { dest: len_dest, func: len_func, args: vec![iter_reg] });

                    // ensure iterator local exists (the loop variable)
                    let locals = self.current_locals_mut();
                    let iter_local_idx = if let Some(i) = locals.get(iterator) { *i } else { let ni = locals.len(); locals.insert(iterator.clone(), ni); ni };

                    // create an index local
                    let idx_local = locals.len();
                    locals.insert(format!("{}_idx", iterator), idx_local);

                    // idx = 0
                    let zero = self.alloc_reg();
                    self.ops.push(IROp::LConst { dest: zero, value: Value::Int(0) });
                    self.ops.push(IROp::SLocal { src: zero, local_index: idx_local });

                    let start_lbl = self.new_label();
                    let end_lbl = self.new_label();
                    self.emit_label(start_lbl.clone());

                    // load idx and compare idx < len_dest
                    let idx_r = self.alloc_reg();
                    self.ops.push(IROp::LLocal { dest: idx_r, local_index: idx_local });
                    let cmp = self.alloc_reg();
                    self.ops.push(IROp::Lt { dest: cmp, src1: idx_r, src2: len_dest });
                    self.emit_brfalse_to(cmp, end_lbl.clone());

                    // val = index(iterable, idx)
                    let index_func = self.alloc_reg();
                    self.ops.push(IROp::LConst { dest: index_func, value: Value::Str("index".to_string()) });
                    let val_dest = self.alloc_reg();
                    self.ops.push(IROp::Call { dest: val_dest, func: index_func, args: vec![iter_reg, idx_r] });
                    // store into iterator local
                    self.ops.push(IROp::SLocal { src: val_dest, local_index: iter_local_idx });

                    // body
                    self.walk_node(body);

                    // idx = idx + 1
                    let one = self.alloc_reg();
                    self.ops.push(IROp::LConst { dest: one, value: Value::Int(1) });
                    let newv = self.alloc_reg();
                    self.ops.push(IROp::Add { dest: newv, src1: idx_r, src2: one });
                    self.ops.push(IROp::SLocal { src: newv, local_index: idx_local });
                    self.emit_jump_to(start_lbl.clone());
                    self.emit_label(end_lbl);
                }
                None
            }
            AstNodeKind::Block { statements } => {
                for s in statements.iter() {
                    self.walk_node(s);
                }
                None
            }
            AstNodeKind::Assignment { target, value } => {
                // Only handle simple identifier targets for now
                let val_reg_opt = self.walk_node(value);
                let val_reg = if let Some(r) = val_reg_opt { r } else { self.alloc_reg() };
                if let AstNodeKind::Identifier { name } = &target.kind {
                    // store into local slot without nested borrows
                    let locals = self.current_locals_mut();
                    let idx = if let Some(existing) = locals.get(name) {
                        *existing
                    } else {
                        let new_idx = locals.len();
                        locals.insert(name.clone(), new_idx);
                        new_idx
                    };
                    self.ops.push(IROp::SLocal { src: val_reg, local_index: idx });
                }
                Some(val_reg)
            }
            AstNodeKind::Return { value } => {
                if let Some(v) = value {
                    let src_opt = self.walk_node(v);
                    let src = if let Some(r) = src_opt { r } else { self.alloc_reg() };
                    self.ops.push(IROp::Ret { src });
                } else {
                    // return null -> push a literal null register then ret
                    let r = self.alloc_reg();
                    self.ops.push(IROp::LConst { dest: r, value: Value::Null });
                    self.ops.push(IROp::Ret { src: r });
                }
                None
            }
            AstNodeKind::Identifier { name } => {
                // load local if present - first find index without mutable borrow
                let mut found_idx: Option<usize> = None;
                for scope in self.locals.iter().rev() {
                    if let Some(idx) = scope.get(name) {
                        found_idx = Some(*idx);
                        break;
                    }
                }
                if let Some(idx) = found_idx {
                    let dest = self.alloc_reg();
                    self.ops.push(IROp::LLocal { dest, local_index: idx });
                    return Some(dest);
                }
                // not found: treat as constant string reference to name
                let dest = self.alloc_reg();
                self.ops.push(IROp::LConst { dest, value: Value::Str(name.clone()) });
                Some(dest)
            }
            AstNodeKind::Integer { value } => {
                let dest = self.alloc_reg();
                self.ops.push(IROp::LConst { dest, value: Value::Int(*value) });
                Some(dest)
            }
            AstNodeKind::Float { value } => {
                let dest = self.alloc_reg();
                self.ops.push(IROp::LConst { dest, value: Value::Float(*value) });
                Some(dest)
            }
            AstNodeKind::Bool { value } => {
                let dest = self.alloc_reg();
                self.ops.push(IROp::LConst { dest, value: Value::Bool(*value) });
                Some(dest)
            }
            AstNodeKind::String { value } => {
                let dest = self.alloc_reg();
                self.ops.push(IROp::LConst { dest, value: Value::Str(value.clone()) });
                Some(dest)
            }
            AstNodeKind::BinaryOp { left, op, right } => {
                let r1_opt = self.walk_node(left);
                let r1 = if let Some(r) = r1_opt { r } else { self.alloc_reg() };
                let r2_opt = self.walk_node(right);
                let r2 = if let Some(r) = r2_opt { r } else { self.alloc_reg() };
                let dest = self.alloc_reg();
                use BinaryOperator::*;
                let ir = match op {
                    Add => IROp::Add { dest, src1: r1, src2: r2 },
                    Sub => IROp::Sub { dest, src1: r1, src2: r2 },
                    Mul => IROp::Mul { dest, src1: r1, src2: r2 },
                    Div => IROp::Div { dest, src1: r1, src2: r2 },
                    Mod => IROp::Mod { dest, src1: r1, src2: r2 },
                    Eq  => IROp::Eq  { dest, src1: r1, src2: r2 },
                    Ne  => IROp::Neq { dest, src1: r1, src2: r2 },
                    Lt  => IROp::Lt  { dest, src1: r1, src2: r2 },
                    Le  => IROp::Lte { dest, src1: r1, src2: r2 },
                    Gt  => IROp::Gt  { dest, src1: r1, src2: r2 },
                    Ge  => IROp::Gte { dest, src1: r1, src2: r2 },
                };
                self.ops.push(ir);
                Some(dest)
            }
            AstNodeKind::UnaryOp { op, expr } => {
                let r_opt = self.walk_node(expr);
                let r = if let Some(rr) = r_opt { rr } else { self.alloc_reg() };
                let dest = self.alloc_reg();
                use UnaryOperator::*;
                let ir = match op {
                    Plus => { self.ops.push(IROp::LConst { dest, value: Value::Int(0) }); IROp::Add { dest, src1: dest, src2: r } }
                    Minus => { self.ops.push(IROp::LConst { dest, value: Value::Int(0) }); IROp::Sub { dest, src1: dest, src2: r } }
                    Not => IROp::Not { dest, src: r },
                    Inc => { IROp::Inc { dest: r } },
                    Dec => { IROp::Dec { dest: r } },
                };
                self.ops.push(ir);
                Some(dest)
            }
            AstNodeKind::Call { callee, args } => {
                // If the callee is a stage identifier and we have its label, emit a direct CallLabel
                if let AstNodeKind::Identifier { name } = &callee.kind {
                    if let Some(&label_idx) = self.stage_labels.get(name) {
                        // lower args
                        let mut arg_regs = Vec::new();
                        for a in args.iter() {
                            if let Some(ar) = self.walk_node(a) {
                                arg_regs.push(ar);
                            }
                        }
                        let dest = self.alloc_reg();
                        self.ops.push(IROp::CallLabel { dest, label_index: label_idx, args: arg_regs });
                        return Some(dest);
                    }
                }

                // Fallback: represent function as a register (string const or evaluated expr)
                let func_reg = if let AstNodeKind::Identifier { name } = &callee.kind {
                    let r = self.alloc_reg();
                    self.ops.push(IROp::LConst { dest: r, value: Value::Str(name.clone()) });
                    r
                } else {
                    let fr_opt = self.walk_node(callee);
                    if let Some(fr) = fr_opt { fr } else { self.alloc_reg() }
                };
                let mut arg_regs = Vec::new();
                for a in args.iter() {
                    if let Some(ar) = self.walk_node(a) {
                        arg_regs.push(ar);
                    }
                }
                let dest = self.alloc_reg();
                self.ops.push(IROp::Call { dest, func: func_reg, args: arg_regs });
                Some(dest)
            }
            AstNodeKind::List { elements } => {
                // Lower elements and pack into an array literal value
                let mut vals = Vec::new();
                for e in elements.iter() {
                    match &e.kind {
                        AstNodeKind::Integer { value } => vals.push(Value::Int(*value)),
                        AstNodeKind::Float { value } => vals.push(Value::Float(*value)),
                        AstNodeKind::Bool { value } => vals.push(Value::Bool(*value)),
                        AstNodeKind::String { value } => vals.push(Value::Str(value.clone())),
                        AstNodeKind::Identifier { name } => {
                            // Preserve identifier as a string value in the array literal
                            vals.push(Value::Str(name.clone()));
                        }
                        _ => {
                            // For complex elements, attempt to evaluate them; if lowering
                            // produces a constant register we cannot capture here, so
                            // fall back to Null to keep existing behavior.
                            self.walk_node(e);
                            vals.push(Value::Null);
                        }
                    }
                }
                let dest = self.alloc_reg();
                self.ops.push(IROp::LConst { dest, value: Value::Array(vals) });
                Some(dest)
            }
            _ => None,
        }
    }
}

use std::fmt;

impl std::fmt::Display for IrModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for op in self.ops.iter() {
            writeln!(f, "{}", op)?;
        }
        Ok(())
    }
}