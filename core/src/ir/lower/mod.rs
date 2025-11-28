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
    /// Map of project name -> member map (members are simple lowered Values)
    projects: HashMap<String, HashMap<String, Value>>,
    /// Map of local name -> static list of identifier elements discovered in a workspace
    static_list_locals: HashMap<String, Vec<String>>,
    /// Map register -> constant Value when the register holds a compile-time literal
    reg_constants: HashMap<usize, Value>,
    /// Map local_index -> constant Value when the local was assigned a compile-time literal
    local_constants: HashMap<usize, Value>,
    /// Map dest register produced by an `LLocal` -> local_index (for tracking origins)
    llocal_map: HashMap<usize, usize>,
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
            projects: HashMap::new(),
            static_list_locals: HashMap::new(),
            reg_constants: HashMap::new(),
            local_constants: HashMap::new(),
            llocal_map: HashMap::new(),
        }
    }

    pub fn get_stage_labels(&self) -> HashMap<String, usize> {
        self.stage_labels.clone()
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

        // Then, find the workspace or project matching entrypoint and lower it.
        // Workspaces may contain both members and logic; projects contain only
        // members. We should lower the container node (workspace/project)
        // itself so that labels (and any nested stages) are emitted correctly.
        let mut fallback: Option<&AstNode> = None;
        if let AstNodeKind::Script { body } = &ast.kind {
            for n in body.iter() {
                match &n.kind {
                    AstNodeKind::Workspace { name, .. } | AstNodeKind::Project { name, .. } => {
                        if fallback.is_none() {
                            fallback = Some(&n);
                        }
                        if name == entrypoint {
                            // Lower the entire workspace/project node (not just its body)
                            self.walk_node(n);
                            // After lowering entrypoint, resolve relocations and return
                            self.patch_relocations();
                            return;
                        }
                    }
                    _ => {}
                }
            }
        }

        // If entrypoint wasn't found, lower the first workspace or project container
        if let Some(cont) = fallback {
            self.walk_node(cont);
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

    /// Allocate a closure object and capture the given local names into it.
    /// Returns the register containing the closure handle.
    pub fn emit_capture_closure(&mut self, capture_names: &[String]) -> usize {
        // allocate closure object
        let clo_reg = self.alloc_reg();
        self.ops.push(IROp::AllocClosure { dest: clo_reg });

        for (i, name) in capture_names.iter().enumerate() {
            // find local index for the name
            let mut found_idx: Option<usize> = None;
            for scope in self.locals.iter().rev() {
                if let Some(idx) = scope.get(name) {
                    found_idx = Some(*idx);
                    break;
                }
            }
            let local_idx = if let Some(idx) = found_idx {
                idx
            } else {
                // if not found, create a new local slot (fallback)
                let m = self.current_locals_mut();
                let ni = m.len();
                m.insert(name.clone(), ni);
                ni
            };

            // load local into a register
            let val_reg = self.alloc_reg();
            self.ops.push(IROp::LLocal { dest: val_reg, local_index: local_idx });
            self.llocal_map.insert(val_reg, local_idx);

            // store into closure field
            self.ops.push(IROp::CStore { closure: clo_reg, field: i, src: val_reg });
        }

        clo_reg
    }

    fn walk_node(&mut self, node: &AstNode) -> Option<usize> {
        match &node.kind {
            AstNodeKind::Script { body } => {
                for n in body.iter() {
                    self.walk_node(n);
                }
                None
            }
            AstNodeKind::Workspace { name, body } => {
                // Emit a label for the workspace so it can be referenced as an entrypoint
                self.emit_label(name.clone());
                    // Enter a new locals scope for this workspace
                    self.push_locals_scope();

                    // Pre-scan the workspace body for assignments of the form
                    // `name = [id1, id2, ...]` so we can unroll `for x in name`
                    // loops when the list is statically known.
                    if let AstNodeKind::Block { statements } = &body.kind {
                        for s in statements.iter() {
                            if let AstNodeKind::Assignment { target, value } = &s.kind {
                                if let AstNodeKind::Identifier { name: tname } = &target.kind {
                                    if let AstNodeKind::List { elements } = &value.kind {
                                        let mut ids = Vec::new();
                                        let mut all_ids = true;
                                        for e in elements.iter() {
                                            if let AstNodeKind::Identifier { name: idn } = &e.kind {
                                                ids.push(idn.clone());
                                            } else {
                                                all_ids = false;
                                                break;
                                            }
                                        }
                                        if all_ids {
                                            self.static_list_locals.insert(tname.clone(), ids);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Lower workspace body (members + logic)
                    self.walk_node(body);

                    self.pop_locals_scope();
                // Ensure workspace returns by halting if none
                self.ops.push(IROp::Halt);
                None
            }
            AstNodeKind::Project { name, body } => {
                // Projects are data containers (members only). Collect simple
                // member literals into the `projects` registry so runtimes can
                // resolve project metadata without emitting runtime ops here.
                let mut members_map: HashMap<String, Value> = HashMap::new();
                if let AstNodeKind::Block { statements } = &body.kind {
                    for s in statements.iter() {
                        if let AstNodeKind::Assignment { target, value } = &s.kind {
                            if let AstNodeKind::Identifier { name: mname } = &target.kind {
                                if let Some(v) = astnode_to_value(value) {
                                    members_map.insert(mname.clone(), v);
                                }
                            }
                        }
                    }
                }
                self.projects.insert(name.clone(), members_map);
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
                    self.llocal_map.insert(lv, idx);
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
                    if let Some(v) = self.reg_constants.get(&newv) {
                        self.local_constants.insert(idx, v.clone());
                    }
                    self.emit_jump_to(start_lbl.clone());
                }

                self.emit_label(end_lbl);
                None
            }
            AstNodeKind::ForIn { iterator, iterable, body } => {
                // If iterable is a literal list, or a statically-known local list,
                // unroll into sequential executions
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
                        if let Some(v) = self.reg_constants.get(&val_reg) {
                            self.local_constants.insert(idx_local, v.clone());
                        }
                        self.walk_node(body);
                    }
                } else if let AstNodeKind::Identifier { name: iter_name } = &iterable.kind {
                    // If this identifier refers to a static list we discovered
                    // earlier in the workspace, unroll using those element ids.
                    if let Some(elems) = self.static_list_locals.get(iter_name).cloned() {
                        let idx_local = {
                            let locals = self.current_locals_mut();
                            if let Some(i) = locals.get(iterator) { *i } else { let ni = locals.len(); locals.insert(iterator.clone(), ni); ni }
                        };
                        for elem_name in elems.iter() {
                            // represent each element as a Symbol of the identifier
                            let r = self.alloc_reg();
                            let v = Value::Symbol(elem_name.clone());
                            self.ops.push(IROp::LConst { dest: r, value: v.clone() });
                            self.reg_constants.insert(r, v);
                            self.ops.push(IROp::SLocal { src: r, local_index: idx_local });
                            if let Some(v) = self.reg_constants.get(&r) {
                                self.local_constants.insert(idx_local, v.clone());
                            }
                            self.walk_node(body);
                        }
                    } else {
                        // General case (runtime helpers)
                        
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
                    if let Some(v) = self.reg_constants.get(&zero) {
                        self.local_constants.insert(idx_local, v.clone());
                    }

                    let start_lbl = self.new_label();
                    let end_lbl = self.new_label();
                    self.emit_label(start_lbl.clone());

                    // load idx and compare idx < len_dest
                    let idx_r = self.alloc_reg();
                    self.ops.push(IROp::LLocal { dest: idx_r, local_index: idx_local });
                    self.llocal_map.insert(idx_r, idx_local);
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
                    if let Some(v) = self.reg_constants.get(&val_dest) {
                        self.local_constants.insert(iter_local_idx, v.clone());
                    }

                    // body
                    self.walk_node(body);

                    // idx = idx + 1
                    let one = self.alloc_reg();
                    self.ops.push(IROp::LConst { dest: one, value: Value::Int(1) });
                    let newv = self.alloc_reg();
                    self.ops.push(IROp::Add { dest: newv, src1: idx_r, src2: one });
                    self.ops.push(IROp::SLocal { src: newv, local_index: idx_local });
                    if let Some(v) = self.reg_constants.get(&newv) {
                        self.local_constants.insert(idx_local, v.clone());
                    }
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
                    if let Some(v) = self.reg_constants.get(&val_reg) {
                        self.local_constants.insert(idx, v.clone());
                    }
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
                    self.llocal_map.insert(dest, idx);
                    return Some(dest);
                }
                // not found: treat as a symbolic reference to the named object
                let dest = self.alloc_reg();
                let val = Value::Symbol(name.clone());
                self.ops.push(IROp::LConst { dest, value: val.clone() });
                self.reg_constants.insert(dest, val);
                Some(dest)
            }
            AstNodeKind::Integer { value } => {
                let dest = self.alloc_reg();
                let val = Value::Int(*value);
                self.ops.push(IROp::LConst { dest, value: val.clone() });
                self.reg_constants.insert(dest, val);
                Some(dest)
            }
            AstNodeKind::Float { value } => {
                let dest = self.alloc_reg();
                let val = Value::Float(*value);
                self.ops.push(IROp::LConst { dest, value: val.clone() });
                self.reg_constants.insert(dest, val);
                Some(dest)
            }
            AstNodeKind::Bool { value } => {
                let dest = self.alloc_reg();
                let val = Value::Bool(*value);
                self.ops.push(IROp::LConst { dest, value: val.clone() });
                self.reg_constants.insert(dest, val);
                Some(dest)
            }
            AstNodeKind::String { value } => {
                let dest = self.alloc_reg();
                let val = Value::Str(value.clone());
                self.ops.push(IROp::LConst { dest, value: val.clone() });
                self.reg_constants.insert(dest, val);
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

                        // Try to substitute any argument that ultimately comes from
                        // a local that was assigned a compile-time constant (e.g. a
                        // project Symbol). If found, replace the arg with a fresh
                        // LConst register so the call receives a constant.
                        // First, collect candidate constant values for each arg without
                        // performing any mutable operations (avoids borrow conflicts).
                        let mut substitutes: Vec<Option<Value>> = Vec::with_capacity(arg_regs.len());
                        for &reg in arg_regs.iter() {
                            if let Some(&local_idx) = self.llocal_map.get(&reg) {
                                substitutes.push(self.local_constants.get(&local_idx).cloned());
                            } else {
                                substitutes.push(self.reg_constants.get(&reg).cloned());
                            }
                        }

                        // Now apply substitutions where we have a constant value.
                        for (i, sub) in substitutes.into_iter().enumerate() {
                            if let Some(val) = sub {
                                let const_r = self.alloc_reg();
                                self.ops.push(IROp::LConst { dest: const_r, value: val.clone() });
                                self.reg_constants.insert(const_r, val);
                                arg_regs[i] = const_r;
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
                    self.ops.push(IROp::LConst { dest: r, value: Value::Symbol(name.clone()) });
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
                            // Preserve identifier as a symbolic reference in the array literal
                            // so downstream passes/runtimes can distinguish named objects
                            vals.push(Value::Symbol(name.clone()));
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
                let val = Value::Array(vals);
                self.ops.push(IROp::LConst { dest, value: val.clone() });
                self.reg_constants.insert(dest, val);
                Some(dest)
            }
            _ => None,
        }
    }

}

use std::fmt;

fn astnode_to_value(node: &AstNode) -> Option<Value> {
    match &node.kind {
        AstNodeKind::Integer { value } => Some(Value::Int(*value)),
        AstNodeKind::Float { value } => Some(Value::Float(*value)),
        AstNodeKind::Bool { value } => Some(Value::Bool(*value)),
        AstNodeKind::String { value } => Some(Value::Str(value.clone())),
        AstNodeKind::Identifier { name } => Some(Value::Symbol(name.clone())),
        AstNodeKind::List { elements } => {
            let mut vals = Vec::new();
            for e in elements.iter() {
                if let Some(v) = astnode_to_value(e) {
                    vals.push(v);
                } else {
                    vals.push(Value::Null);
                }
            }
            Some(Value::Array(vals))
        }
        _ => None,
    }
}

impl std::fmt::Display for IrModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Build a mapping from op index -> label name for human-friendly printing
        let mut label_pos: HashMap<usize, String> = HashMap::new();
        for (i, op) in self.ops.iter().enumerate() {
            if let IROp::Label { name } = op {
                label_pos.insert(i, name.clone());
            }
        }

        for op in self.ops.iter() {
            match op {
                IROp::LConst { dest, value } => {
                    match value {
                        Value::Int(v) => writeln!(f, "LConst r{} <- Int({})", dest, v)?,
                        Value::Float(v) => writeln!(f, "LConst r{} <- Float({})", dest, v)?,
                        Value::Bool(v) => writeln!(f, "LConst r{} <- Bool({})", dest, v)?,
                        Value::Str(s) => writeln!(f, "LConst r{} <- Str(\"{}\")", dest, s)?,
                        Value::Symbol(s) => {
                            if self.stage_labels.contains_key(s) {
                                writeln!(f, "LConst r{} <- Symbol(stage:{})", dest, s)?;
                            } else {
                                writeln!(f, "LConst r{} <- Symbol({})", dest, s)?;
                            }
                        }
                        Value::Array(arr) => writeln!(f, "LConst r{} <- Array({:?})", dest, arr)?,
                        Value::Null => writeln!(f, "LConst r{} <- Null", dest)?,
                    }
                }
                IROp::LLocal { dest, local_index } => writeln!(f, "LLocal r{} <- local[{}]", dest, local_index)?,
                IROp::SLocal { src, local_index } => writeln!(f, "SLocal local[{}] <- r{}", local_index, src)?,
                IROp::Add { dest, src1, src2 } => writeln!(f, "Add r{} <- r{} + r{}", dest, src1, src2)?,
                IROp::Sub { dest, src1, src2 } => writeln!(f, "Sub r{} <- r{} - r{}", dest, src1, src2)?,
                IROp::Mul { dest, src1, src2 } => writeln!(f, "Mul r{} <- r{} * r{}", dest, src1, src2)?,
                IROp::Div { dest, src1, src2 } => writeln!(f, "Div r{} <- r{} / r{}", dest, src1, src2)?,
                IROp::Mod { dest, src1, src2 } => writeln!(f, "Mod r{} <- r{} % r{}", dest, src1, src2)?,
                IROp::Eq { dest, src1, src2 } => writeln!(f, "Eq r{} <- r{} == r{}", dest, src1, src2)?,
                IROp::Neq { dest, src1, src2 } => writeln!(f, "Neq r{} <- r{} != r{}", dest, src1, src2)?,
                IROp::Lt { dest, src1, src2 } => writeln!(f, "Lt r{} <- r{} < r{}", dest, src1, src2)?,
                IROp::Lte { dest, src1, src2 } => writeln!(f, "Lte r{} <- r{} <= r{}", dest, src1, src2)?,
                IROp::Gt { dest, src1, src2 } => writeln!(f, "Gt r{} <- r{} > r{}", dest, src1, src2)?,
                IROp::Gte { dest, src1, src2 } => writeln!(f, "Gte r{} <- r{} >= r{}", dest, src1, src2)?,
                IROp::And { dest, src1, src2 } => writeln!(f, "And r{} <- r{} && r{}", dest, src1, src2)?,
                IROp::Or { dest, src1, src2 } => writeln!(f, "Or r{} <- r{} || r{}", dest, src1, src2)?,
                IROp::Not { dest, src } => writeln!(f, "Not r{} <- !r{}", dest, src)?,
                IROp::Inc { dest } => writeln!(f, "Inc r{} ++", dest)?,
                IROp::Dec { dest } => writeln!(f, "Dec r{} --", dest)?,
                IROp::Label { name } => writeln!(f, "Label {}", name)?,
                IROp::Jump { target } => writeln!(f, "Jump {}", target)?,
                IROp::BrTrue { cond, target } => writeln!(f, "BrTrue r{} -> {}", cond, target)?,
                IROp::BrFalse { cond, target } => writeln!(f, "BrFalse r{} -> {}", cond, target)?,
                IROp::Halt => writeln!(f, "Halt")?,
                IROp::AllocClosure { dest } => writeln!(f, "AllocClosure r{}", dest)?,
                IROp::CStore { closure, field, src } => writeln!(f, "CStore clo[r{}].{} <- r{}", closure, field, src)?,
                IROp::CLoad { dest, closure, field } => writeln!(f, "CLoad r{} <- clo[r{}].{}", dest, closure, field)?,
                IROp::Call { dest, func, args } => {
                    write!(f, "Call r{} <- r{}(", dest, func)?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "r{}", arg)?;
                    }
                    writeln!(f, ")")?;
                }
                IROp::CallLabel { dest, label_index, args } => {
                    // Try to resolve the label index to a human-friendly name
                    let name = label_pos.get(label_index).cloned().unwrap_or_else(|| format!("L{}", label_index));
                    write!(f, "CallLabel r{} <- {}(", dest, name)?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "r{}", arg)?;
                    }
                    writeln!(f, ")")?;
                }
                IROp::Ret { src } => writeln!(f, "Ret r{}", src)?,
            }
        }
        Ok(())
    }
}
