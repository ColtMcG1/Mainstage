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
    /// Preserve the AST for top-level stages so we can specialize/inline them
    stage_asts: HashMap<String, AstNode>,
    /// Map stage name -> parameter names (in order)
    stage_params: HashMap<String, Vec<String>>,
    /// Map stage name -> parameter local indices (in the stage's locals scope)
    stage_param_local_indices: HashMap<String, Vec<usize>>,
    /// Map label op index -> label name (reverse of `stage_labels`)
    label_index_to_name: HashMap<usize, String>,
    /// Map original op index (SLocal) -> variable name stored
    op_slocal_name: HashMap<usize, String>,
    /// Map CallLabel op index -> arg name hints (Some(name) if arg was an Identifier)
    callsite_arg_names: HashMap<usize, Vec<Option<String>>>,
    /// When true, `emit_label` will not register stage labels into `stage_labels`
    inlining: bool,
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
            stage_asts: HashMap::new(),
            stage_params: HashMap::new(),
            stage_param_local_indices: HashMap::new(),
            label_index_to_name: HashMap::new(),
            op_slocal_name: HashMap::new(),
            callsite_arg_names: HashMap::new(),
            inlining: false,
        }
    }

    pub fn get_stage_labels(&self) -> HashMap<String, usize> {
        self.stage_labels.clone()
    }

    pub fn get_callsite_arg_names(&self) -> HashMap<usize, Vec<Option<String>>> {
        self.callsite_arg_names.clone()
    }

    pub fn get_stage_param_names(&self, name: &str) -> Option<Vec<String>> {
        self.stage_params.get(name).cloned()
    }

    pub fn get_stage_param_local_indices(&self, name: &str) -> Option<Vec<usize>> {
        self.stage_param_local_indices.get(name).cloned()
    }

    pub fn get_op_slocal_name(&self) -> HashMap<usize, String> {
        self.op_slocal_name.clone()
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

    fn local_name_by_index(&self, idx: usize) -> Option<String> {
        for scope in self.locals.iter().rev() {
            for (k, &v) in scope.iter() {
                if v == idx {
                    return Some(k.clone());
                }
            }
        }
        None
    }

    fn emit_slocal(&mut self, src: usize, local_index: usize) {
        let pos = self.ops.len();
        self.ops.push(IROp::SLocal { src, local_index });
        if let Some(name) = self.local_name_by_index(local_index) {
            self.op_slocal_name.insert(pos, name);
        }
        if let Some(v) = self.reg_constants.get(&src) {
            self.local_constants.insert(local_index, v.clone());
        }
    }

    pub fn lower_from_ast(&mut self, ast: &AstNode, entrypoint: &str) {
        // First, lower project declarations so their member data is available
        // to later lowering of workspaces/stages.
        if let AstNodeKind::Script { body } = &ast.kind {
            for n in body.iter() {
                if let AstNodeKind::Project { .. } = &n.kind {
                    self.walk_node(n);
                }
            }
        }

        // Next, lower stage declarations so their labels exist for calls.
        if let AstNodeKind::Script { body } = &ast.kind {
            for n in body.iter() {
                if let AstNodeKind::Stage { name, .. } = &n.kind {
                    // Record the stage AST so we can specialize/inline it later.
                    self.stage_asts.insert(name.clone(), n.clone());
                    // Also lower the stage now as usual so non-specialized
                    // runs still have a global callable version.
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
                            // Ensure execution starts at the selected entrypoint label by
                            // inserting a Jump at position 0 that relocates to the label.
                            // When we insert at the front, all previously-recorded relocation
                            // positions must be incremented to remain valid.
                            self.ops.insert(0, IROp::Jump { target: 0 });
                            for rel in self.relocations.iter_mut() {
                                rel.0 += 1;
                            }
                            self.relocations.push((0, name.clone()));
                            // After lowering entrypoint, resolve relocations and return
                            self.patch_relocations();
                            return; // Ensure only one return statement
                        }
                    }
                    _ => {}
                }
            }
        }

        // If entrypoint wasn't found, lower the first workspace or project container
        if let Some(cont) = fallback {
            // extract its name to make it the implicit entrypoint
            let cont_name = match &cont.kind {
                AstNodeKind::Workspace { name, .. } => name.clone(),
                AstNodeKind::Project { name, .. } => name.clone(),
                _ => "entry".to_string(),
            };
            self.walk_node(cont);
            // Prepend a Jump to the chosen container label so bytecode execution
            // begins at the intended entrypoint.
            self.ops.insert(0, IROp::Jump { target: 0 });
            for rel in self.relocations.iter_mut() {
                rel.0 += 1;
            }
            self.relocations.push((0, cont_name));
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
        // Only register top-level stage labels when not inlining specialized
        // bodies. When inlining we still emit a Label op but do not treat it
        // as a top-level stage label mapping.
        if !self.inlining {
            self.stage_labels.insert(name.clone(), pos);
            self.label_index_to_name.insert(pos, name);
        }
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
                        self.ops[*pos] = IROp::Jump { target: target_idx+1 };
                    }
                    IROp::BrFalse { cond, .. } => {
                        let c = *cond;
                        // Branch targets should also account for the prepended Jump
                        self.ops[*pos] = IROp::BrFalse { cond: c, target: target_idx+1 };
                    }
                    IROp::BrTrue { cond, .. } => {
                        let c = *cond;
                        // Branch targets should also account for the prepended Jump
                        self.ops[*pos] = IROp::BrTrue { cond: c, target: target_idx+1 };
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
                        let mut param_names: Vec<String> = Vec::new();
                        let mut param_indices: Vec<usize> = Vec::new();
                        for p in params.iter() {
                            if let AstNodeKind::Identifier { name: pname } = &p.kind {
                                let idx = locals.len();
                                locals.insert(pname.clone(), idx);
                                param_names.push(pname.clone());
                                param_indices.push(idx);
                            }
                        }
                        // record param metadata for optimizer
                        self.stage_params.insert(name.clone(), param_names);
                        self.stage_param_local_indices.insert(name.clone(), param_indices);
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
            AstNodeKind::Member { object, property } => {
                // Try to resolve compile-time project member access like `prj.sources`
                if let AstNodeKind::Identifier { name: obj_name } = &object.kind {
                    // `property` is a string (the member name)
                    let prop_name = property.clone();

                    // First, if `obj_name` refers to a local with a compile-time constant
                    // symbol (e.g. a parameter `prj` seeded to `Symbol(test_pj)` during
                    // inlining), resolve via local_constants -> projects lookup.
                    let mut resolved = false;
                    for scope in self.locals.iter().rev() {
                        if let Some(&local_idx) = scope.get(obj_name) {
                            if let Some(v) = self.local_constants.get(&local_idx) {
                                if let Value::Symbol(proj_name) = v {
                                    if let Some(members) = self.projects.get(proj_name).cloned() {
                                        if let Some(mv) = members.get(&prop_name) {
                                            let dest = self.alloc_reg();
                                            self.ops.push(IROp::LConst { dest, value: mv.clone() });
                                            self.reg_constants.insert(dest, mv.clone());
                                            return Some(dest);
                                        }
                                    }
                                    resolved = true;
                                    break;
                                }
                            }
                        }
                    }

                    if !resolved {
                        // fallback: object is a top-level identifier referencing a project name
                        if let Some(members) = self.projects.get(obj_name).cloned() {
                            if let Some(v) = members.get(&prop_name) {
                                let dest = self.alloc_reg();
                                self.ops.push(IROp::LConst { dest, value: v.clone() });
                                self.reg_constants.insert(dest, v.clone());
                                return Some(dest);
                            }
                        }
                    }
                }
                // Fallback: not implemented yet — return None so callers treat as runtime value
                None
            }
            AstNodeKind::Index { object, index } => {
                // Try to resolve compile-time index into constant arrays, e.g. `prj.sources[0]`
                // First, try to evaluate the object to a constant register
                if let Some(obj_reg) = self.walk_node(object) {
                    // if the object register is a known constant array, and index is integer literal,
                    // fold to the element value
                    if let Some(val) = self.reg_constants.get(&obj_reg).cloned() {
                        if let Value::Array(arr) = val {
                            if let AstNodeKind::Integer { value } = &index.kind {
                                let idx = *value as usize;
                                if idx < arr.len() {
                                    let dest = self.alloc_reg();
                                    let v = arr[idx].clone();
                                    self.ops.push(IROp::LConst { dest, value: v.clone() });
                                    self.reg_constants.insert(dest, v);
                                    return Some(dest);
                                }
                            }
                        }
                    }
                }
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
                    self.emit_slocal(newv, idx);
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
                        self.emit_slocal(val_reg, idx_local);
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
                            self.emit_slocal(r, idx_local);
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
                    self.emit_slocal(zero, idx_local);

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
                    self.emit_slocal(val_dest, iter_local_idx);

                    // body
                    self.walk_node(body);

                    // idx = idx + 1
                    let one = self.alloc_reg();
                    self.ops.push(IROp::LConst { dest: one, value: Value::Int(1) });
                    let newv = self.alloc_reg();
                    self.ops.push(IROp::Add { dest: newv, src1: idx_r, src2: one });
                    self.emit_slocal(newv, idx_local);
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
                    self.emit_slocal(val_reg, idx);
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
                // If the callee is a stage identifier and we have its label, try a direct CallLabel
                if let AstNodeKind::Identifier { name } = &callee.kind {
                    if let Some(&label_idx) = self.stage_labels.get(name) {
                        // prepare arg name hints (so optimizer can match identifier args to SLocal stores)
                        let mut arg_name_hints: Vec<Option<String>> = args.iter().map(|a| {
                            if let AstNodeKind::Identifier { name } = &a.kind { Some(name.clone()) } else { None }
                        }).collect();

                        // lower args into registers. Guarantee that when the AST
                        // argument is a bare Identifier we always emit an explicit
                        // register (either an `LLocal` or an `LConst(Symbol)`)
                        // so downstream optimizer can match by name.
                        let mut arg_regs = Vec::new();
                        for a in args.iter() {
                            // If lowering yields a register, use it. Otherwise,
                            // for identifier args synthesize a small literal
                            // register so the callsite has an explicit operand.
                            let ar = match self.walk_node(a) {
                                Some(r) => r,
                                None => {
                                    // If the AST arg is an identifier, represent it
                                    // as a Symbol constant so the optimizer can see
                                    // and match the name. Otherwise allocate a
                                    // fresh register to preserve ordering.
                                    if let AstNodeKind::Identifier { name } = &a.kind {
                                        let rr = self.alloc_reg();
                                        self.ops.push(IROp::LConst { dest: rr, value: Value::Symbol(name.clone()) });
                                        self.reg_constants.insert(rr, Value::Symbol(name.clone()));
                                        rr
                                    } else {
                                        self.alloc_reg()
                                    }
                                }
                            };
                            arg_regs.push(ar);
                        }

                        // Collect candidate constant substitutions (without mutating) to avoid borrow issues
                        let mut substitutes: Vec<Option<Value>> = Vec::with_capacity(arg_regs.len());
                        for &reg in arg_regs.iter() {
                            if let Some(&local_idx) = self.llocal_map.get(&reg) {
                                substitutes.push(self.local_constants.get(&local_idx).cloned());
                            } else {
                                substitutes.push(self.reg_constants.get(&reg).cloned());
                            }
                        }

                        // Apply substitutions: replace arg reg with fresh LConst register when constant
                        for (i, sub) in substitutes.into_iter().enumerate() {
                            if let Some(val) = sub {
                                let const_r = self.alloc_reg();
                                self.ops.push(IROp::LConst { dest: const_r, value: val.clone() });
                                self.reg_constants.insert(const_r, val);
                                arg_regs[i] = const_r;
                            }
                        }

                        // Attempt conservative inlining/specialization: only when we have the stage AST
                        if let Some(stage_ast) = self.stage_asts.get(name).cloned() {
                            // collect parameter names
                            let mut param_names: Vec<String> = Vec::new();
                            if let AstNodeKind::Stage { args: maybe_args, .. } = &stage_ast.kind {
                                if let Some(params_node) = maybe_args {
                                    if let AstNodeKind::Arguments { args: params } = &params_node.kind {
                                        for p in params.iter() {
                                            if let AstNodeKind::Identifier { name: pname } = &p.kind {
                                                param_names.push(pname.clone());
                                            }
                                        }
                                    }
                                }
                            }

                            // Determine constant bindings for parameters (from arg_regs)
                            let mut param_bindings: HashMap<String, Value> = HashMap::new();
                            for (i, pname) in param_names.iter().enumerate() {
                                if i < arg_regs.len() {
                                    let reg = arg_regs[i];
                                    if let Some(v) = self.reg_constants.get(&reg) {
                                        param_bindings.insert(pname.clone(), v.clone());
                                    }
                                }
                            }

                            // Conservative specialization: if a parameter is bound to a project
                            // symbol and that project has a static `sources` array whose
                            // first element is a string, substitute the argument register
                            // with an LConst containing that string. This makes calls like
                            // `load_stage(prj.sources[0])` receive the literal path.
                            if !param_names.is_empty() {
                                // We'll operate on a copy of arg_regs to avoid borrow issues
                                let mut new_arg_regs = arg_regs.clone();
                                for (i, pname) in param_names.iter().enumerate() {
                                    if i >= new_arg_regs.len() { break; }
                                    // skip if already a constant binding
                                    if param_bindings.contains_key(pname) { continue; }

                                    let areg = new_arg_regs[i];
                                    // try to find a symbol value for this arg (either reg constant
                                    // or originating local constant via llocal_map)
                                    let mut sym_opt: Option<String> = None;
                                    if let Some(v) = self.reg_constants.get(&areg) {
                                        if let Value::Symbol(s) = v {
                                            sym_opt = Some(s.clone());
                                        }
                                    }
                                    if sym_opt.is_none() {
                                        if let Some(&local_idx) = self.llocal_map.get(&areg) {
                                            if let Some(v) = self.local_constants.get(&local_idx) {
                                                if let Value::Symbol(s) = v {
                                                    sym_opt = Some(s.clone());
                                                }
                                            }
                                        }
                                    }

                                    if let Some(proj_name) = sym_opt {
                                        // clone members to avoid holding an immutable borrow of self
                                        if let Some(members) = self.projects.get(&proj_name).cloned() {
                                            if let Some(Value::Array(arr)) = members.get("sources") {
                                                if let Some(first) = arr.get(0) {
                                                    if let Value::Str(pat) = first {
                                                        // insert an LConst for the string and substitute arg
                                                        let const_r = self.alloc_reg();
                                                        let val = Value::Str(pat.clone());
                                                        self.ops.push(IROp::LConst { dest: const_r, value: val.clone() });
                                                        self.reg_constants.insert(const_r, val);
                                                        new_arg_regs[i] = const_r;
                                                        // also record a param binding so inlining can see it
                                                        param_bindings.insert(pname.clone(), Value::Str(pat.clone()));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // replace arg_regs with substituted version
                                arg_regs = new_arg_regs;

                                // If we know the callee's parameter names and some parameters were
                                // not provided as explicit args, try to synthesize argument registers
                                // by loading caller locals with matching names. This makes callsites
                                // that used implicit local-based parameter passing become explicit
                                // CallLabel args so optimizer can see constant values.
                                if let Some(stage_ast) = self.stage_asts.get(name).cloned() {
                                    let mut param_names: Vec<String> = Vec::new();
                                    if let AstNodeKind::Stage { args: maybe_args, .. } = &stage_ast.kind {
                                        if let Some(params_node) = maybe_args {
                                            if let AstNodeKind::Arguments { args: params } = &params_node.kind {
                                                for p in params.iter() {
                                                    if let AstNodeKind::Identifier { name: pname } = &p.kind {
                                                        param_names.push(pname.clone());
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if !param_names.is_empty() && arg_regs.len() < param_names.len() {
                                        for pn in param_names.iter().skip(arg_regs.len()) {
                                            // search caller locals for this name
                                            let mut found_idx: Option<usize> = None;
                                            for scope in self.locals.iter().rev() {
                                                if let Some(idx) = scope.get(pn) {
                                                    found_idx = Some(*idx);
                                                    break;
                                                }
                                            }
                                            if let Some(li) = found_idx {
                                                let r = self.alloc_reg();
                                                self.ops.push(IROp::LLocal { dest: r, local_index: li });
                                                self.llocal_map.insert(r, li);
                                                arg_regs.push(r);
                                                arg_name_hints.push(Some(pn.clone()));
                                            } else {
                                                // no matching caller local; leave missing
                                            }
                                        }
                                    }
                                }
                            }

                            // Only inline if all parameters are constants
                            if param_bindings.len() == param_names.len() {
                                let dest = self.alloc_reg();

                                // Push locals scope and register parameter locals seeded with constants
                                self.push_locals_scope();
                                if !param_names.is_empty() {
                                    // Insert parameter names into locals, collecting seeded constants
                                    let mut seeded: Vec<(usize, Value)> = Vec::new();
                                    {
                                        let locals = self.current_locals_mut();
                                        for pname in param_names.iter() {
                                            let idx = locals.len();
                                            locals.insert(pname.clone(), idx);
                                            if let Some(v) = param_bindings.get(pname) {
                                                seeded.push((idx, v.clone()));
                                            }
                                        }
                                    }
                                    // Now that the locals borrow is dropped, populate local_constants
                                    for (idx, v) in seeded.into_iter() {
                                        self.local_constants.insert(idx, v);
                                    }
                                }

                                // Create temp return local
                                let ret_local_idx = self.current_locals_mut().len();
                                self.current_locals_mut().insert("__inl_ret".to_string(), ret_local_idx);

                                // Inline the callee body without registering its top-level labels
                                if let AstNodeKind::Stage { body, .. } = &stage_ast.kind {
                                    self.inlining = true;
                                    let start = self.ops.len();
                                    self.walk_node(body);
                                    let end = self.ops.len();
                                    self.inlining = false;

                                    // Prepare after-inline label and patch any Ret -> store+jump
                                    let after_lbl = self.new_label();
                                    let mut ret_positions: Vec<usize> = Vec::new();
                                    for i in start..end {
                                        if let IROp::Ret { .. } = &self.ops[i] {
                                            ret_positions.push(i);
                                        }
                                    }

                                    let mut offset = 0;
                                    for pos in ret_positions.iter() {
                                        let p = *pos + offset;
                                        if let IROp::Ret { src } = self.ops[p].clone() {
                                            self.ops[p] = IROp::SLocal { src, local_index: ret_local_idx };
                                            let jpos = p + 1;
                                            self.ops.insert(jpos, IROp::Jump { target: 0 });
                                            self.relocations.push((jpos, after_lbl.clone()));
                                            offset += 1;
                                        }
                                    }

                                    // Load return local into dest and emit after-inline label
                                    self.ops.push(IROp::LLocal { dest, local_index: ret_local_idx });
                                    self.emit_label(after_lbl);
                                }

                                self.pop_locals_scope();
                                return Some(dest);
                            }
                        }

                        // Not specialized/inlined: emit a normal CallLabel
                        let dest = self.alloc_reg();
                        let pos = self.ops.len();
                        self.ops.push(IROp::CallLabel { dest, label_index: label_idx, args: arg_regs });
                        // record arg name hints for optimizer
                        self.callsite_arg_names.insert(pos, arg_name_hints);
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
