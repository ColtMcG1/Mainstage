use crate::ir::op::IROp;
// intentionally reference lower_stmt via the module path where needed

pub fn lower_script_objects(
    script: &crate::ast::AstNode,
    ir_mod: &mut crate::ir::module::IrModule,
    analysis: Option<&crate::analyzers::output::AnalyzerOutput>,
) {

    // Create a lowering context from analyzer output if provided so lowering
    // can use pre-resolved symbols and prototypes.
    let mut ctx = match analysis {
        Some(a) => super::lowering_context::LoweringContext::from_analyzer_output(a, ir_mod),
        None => super::lowering_context::LoweringContext::new(),
    };

    // Ensure a small set of builtin runtime functions are declared so calls
    // like `say(...)` and `read(...)` lower to CallLabel rather than
    // falling back to Null producers.
    super::declare_builtins::declare_builtin_functions(ir_mod, &mut ctx);

    if let Some(body) = match script.get_kind() {
        crate::ast::AstNodeKind::Script { body, .. } => Some(body),
        _ => None,
    } {
        // First pass: register prototypes. Register projects first so that
        // stages (which may reference project prototypes) can rely on
        // project symbols being present during later lowering.
        for stmt in body.iter() {
            if let crate::ast::AstNodeKind::Project { .. } = stmt.get_kind() {
                lower_project_object(stmt, ir_mod, &mut ctx);
            }
        }
        // Then register stages (function prototypes)
        for stmt in body.iter() {
            if let crate::ast::AstNodeKind::Stage { .. } = stmt.get_kind() {
                lower_stage_object(stmt, ir_mod, &mut ctx);
            }
        }

        // If analyzer provided entry_points, ensure any workspace entrypoints
        // are declared as functions so later lowering can emit entrypoint
        // wrappers and module-level calls. This bridges analyzer output
        // (which may mark a workspace as an entrypoint) to the IR module.
        if let Some(a) = analysis {
            for stmt in body.iter() {
                if let crate::ast::AstNodeKind::Workspace { name, .. } = stmt.get_kind() {
                    if a.entry_point == stmt.get_id() {
                        if ctx.get_function_id(stmt.get_id()).is_none() {
                            let func_name = name.clone();
                            let fid = ir_mod.declare_function(&func_name);
                            ctx.bind_function_id(stmt.get_id(), fid);
                            // also populate symbols so calls can resolve by name
                            ctx.symbols.insert(func_name, fid);
                        }
                    }
                }
            }
        }

        // Second pass: walk the AST and emit simple CallLabel ops for Call nodes
        for stmt in body.iter() {
            match stmt.get_kind() {
                // Now perform workspace lowering in the second pass so that
                // project declarations and other prototypes are already in
                // the context.
                crate::ast::AstNodeKind::Workspace { .. } => lower_workspace_object(stmt, ir_mod, &mut ctx),
                // Projects were handled in first pass; skip them here.
                crate::ast::AstNodeKind::Project { .. } => { continue; }
                // Stages still need their function bodies emitted.
                _ => emit_calls_in_node(stmt, ir_mod, &ctx),
            }
        }
    }
    // Resolve any deferred branch targets (labels emitted later)
    ir_mod.patch_unresolved_branches();
    // Emit a final Halt to terminate execution cleanly
    ir_mod.emit_op(IROp::Halt);
}

fn emit_calls_in_node(
    node: &crate::ast::AstNode,
    ir_mod: &mut crate::ir::module::IrModule,
    ctx: &super::lowering_context::LoweringContext,
) {
    use super::function_builder::FunctionBuilder;

    match node.get_kind() {
        // If this kind is a container (workspace/project/stage), recurse into its body.
        // For stage nodes, create a per-function `FunctionBuilder` so registers
        // allocated while lowering the stage are local to that stage.
        k if k.container_body().is_some() => {
            if let Some(body) = k.container_body() {
                if let crate::ast::AstNodeKind::Stage { name, .. } = k {
                    // If we have a bound function id, emit the label inside the
                    // function builder so labels and body ops stay together.
                    let mut fb = FunctionBuilder::new();
                    // Pre-create locals for function parameters so VM arg seeding
                    // (which places args into frame.locals[0..]) lines up with
                    // these local indices.
                    if let Some(params) = ctx.functions_params.get(&node.get_id()) {
                        for p in params.iter() {
                            fb.get_or_create_local(p);
                        }
                    }
                    if let Some(id) = ctx.symbols.get(name).copied() {
                        let label_idx = (id as usize).saturating_sub(1);
                        let label_name = format!("L{}", label_idx);
                        fb.emit_op(IROp::Label { name: label_name });
                    }
                    // Lower the body using the function builder
                    super::lower_stmt::emit_calls_in_node_with_builder(body, &mut fb, ir_mod, ctx);
                    // finalize into the module
                    fb.finalize_into(ir_mod);
                    return;
                }

                // Non-stage containers: just descend
                emit_calls_in_node(body, ir_mod, ctx);
                return;
            }
        }
        _ => {
            // Delegate all other node kinds to the statement-level lowering
            // which will use expression lowering helpers as needed.
            super::lower_stmt::lower_statment(node, ir_mod, ctx);
        }
    }
}



/// Workspace object lowering. Can contain both members and logic.
fn lower_workspace_object(
    workspace_node: &crate::ast::AstNode,
    ir_mod: &mut crate::ir::module::IrModule,
    ctx: &mut super::lowering_context::LoweringContext,
) {
    let body = workspace_node
        .get_kind()
        .container_body()
        .expect("Workspace node should have a body");

    // If this workspace has a name, declare it as an object so other lowering
    // can refer to it (and so analyzer-backed contexts map names -> ids).
    if let crate::ast::AstNodeKind::Workspace { name, .. } = workspace_node.get_kind() {
        if ctx.get_object_id(workspace_node.get_id()).is_none() {
            let oid = ir_mod.declare_object(name);
            ctx.bind_object_id(workspace_node.get_id(), oid);
            ctx.symbols.insert(name.clone(), oid);
        }
    }

    // Collect the workspace body statements (do not lower them yet) so we
    // can detect project declarations and static list assignments to emit
    // array constants and wire iteration lowering. While collecting we
    // suppress module-level emission so any side-effecting statements that
    // belong to the workspace body aren't emitted at module scope.
    ctx.push_suppress_module_emits();
    let members = collect_member_definitions(body, ir_mod, ctx);
    ctx.pop_suppress_module_emits();

    // list collected members (debug removed)

    // First pass: emit array constants for assignments like `ident = [a, b]`
    for stmt in members.iter() {
        if let crate::ast::AstNodeKind::Assignment { target, value } = stmt.get_kind() {
                    if let crate::ast::AstNodeKind::Identifier { .. } = target.get_kind() {
                if let crate::ast::AstNodeKind::List { elements } = value.get_kind() {
                    // Attempt to build an array of the actual project object runtime
                    // registers. Prefer emitting an `ArrayNew` that references the
                    // object regs directly so consumers get real objects, not
                    // bare symbols. Fall back to the previous Symbol-valued
                    // constant array if any element cannot be resolved to a
                    // runtime object register.
                    let mut elem_regs: Vec<usize> = Vec::new();
                    let mut fallback_items: Vec<crate::ir::value::Value> = Vec::new();
                    let mut all_resolved = true;
                    for el in elements.iter() {
                        if let crate::ast::AstNodeKind::Identifier { name } = el.get_kind() {
                            // try to resolve the identifier name to a declared
                            // object id via the lowering context symbol map and
                            // then to a runtime register holding that object.
                            if let Some(&obj_id) = ctx.symbols.get(name) {
                                if let Some(obj_reg) = ctx.get_object_reg_by_objid(obj_id) {
                                    elem_regs.push(obj_reg);
                                    continue;
                                }
                            }
                            // couldn't resolve to a runtime object reg; record
                            // a Symbol fallback and mark as not fully resolved.
                            fallback_items.push(crate::ir::value::Value::Symbol(name.clone()));
                            all_resolved = false;
                        } else {
                            all_resolved = false;
                            break;
                        }
                    }
                    if all_resolved && !elem_regs.is_empty() {
                        // Emit an ArrayNew that constructs the array at runtime
                        // from the element registers (which reference project
                        // object runtime slots).
                        let arr_reg = ir_mod.alloc_reg();
                        ir_mod.emit_op(IROp::ArrayNew { dest: arr_reg, elems: elem_regs });
                        ctx.bind_list_array(target.get_id(), arr_reg);
                        // don't lower this assignment later
                    } else if !fallback_items.is_empty() {
                        // Fall back to previous behavior: emit a constant array
                        // of Symbols if we couldn't resolve all elements to
                        // object regs.
                        let arr_val = crate::ir::value::Value::Array(fallback_items);
                        let arr_reg = ir_mod.alloc_reg();
                        ir_mod.emit_op(IROp::LConst { dest: arr_reg, value: arr_val });
                        ctx.bind_list_array(target.get_id(), arr_reg);
                    }
                }
            }
        }
    }

    // Second pass: emit iteration lowering for `for x in ident` where `ident`
    // refers to one of the statically-created array registers above. Other
    // statements are lowered normally (projects will be handled by
    // `lower_project_object` in the first pass of `lower_script_objects`).
    for stmt in members.iter() {
        // skip project declarations (handled earlier) and static list assigns
        if let crate::ast::AstNodeKind::Project { .. } = stmt.get_kind() { continue; }
                if let crate::ast::AstNodeKind::Assignment { target, value } = stmt.get_kind() {
                    if let crate::ast::AstNodeKind::Identifier { .. } = target.get_kind() {
                        if let crate::ast::AstNodeKind::List { .. } = value.get_kind() {
                            if ctx.get_list_array(target.get_id()).is_some() { continue; }
                        }
                    }
                }

        // Detect `for <ident> in <iterable> { body }` and lower to a simple
        // index-based loop over the array register we emitted above.
        if let crate::ast::AstNodeKind::ForIn { iterator, iterable, body } = stmt.get_kind() {
            if let crate::ast::AstNodeKind::Identifier { .. } = iterable.get_kind() {
                if let Some(arr_reg) = ctx.get_list_array(iterable.get_id()) {
                    // Emit: idx = 0
                    let idx_reg = ir_mod.alloc_reg();
                    ir_mod.emit_op(IROp::LConst { dest: idx_reg, value: crate::ir::value::Value::Int(0) });

                    // Emit: key = Str("length"); len = GetProp arr[key]
                    let key_reg = ir_mod.alloc_reg();
                    ir_mod.emit_op(IROp::LConst { dest: key_reg, value: crate::ir::value::Value::Str("length".to_string()) });
                    let len_reg = ir_mod.alloc_reg();
                    ir_mod.emit_op(IROp::GetProp { dest: len_reg, obj: arr_reg, key: key_reg });

                    // loop_cond: cmp = idx < len
                    let loop_cond_pos = ir_mod.len();
                    let cmp_reg = ir_mod.alloc_reg();
                    ir_mod.emit_op(IROp::Lt { dest: cmp_reg, src1: idx_reg, src2: len_reg });
                    // placeholder BrFalse (will be patched to a generated label)
                    let br_pos = ir_mod.len();
                    // create a unique after-loop label name to resolve later
                    let after_label = format!("__after_ws_{}_{}", workspace_node.get_id(), br_pos);
                    ir_mod.emit_op(IROp::BrFalse { cond: cmp_reg, target: 0 });
                    // record unresolved branch pointing to our after-label
                    ir_mod.record_unresolved_branch(br_pos, after_label.clone());

                    // body: item = ArrayGet arr[idx]
                    let item_reg = ir_mod.alloc_reg();
                    ir_mod.emit_op(IROp::ArrayGet { dest: item_reg, array: arr_reg, index: idx_reg });

                    // Bind the iterator name to the item register temporarily
                    // so any lowering that happens in module-context can still
                    // resolve references to the iterator identifier.
                    ctx.bind_temp_ident(iterator, item_reg);

                    // Create a wrapper function to contain the loop body so the
                    // loop variable can be a real function-local binding. This
                    // avoids complex module-level name binding and keeps semantics
                    // predictable: for each iteration we `CallLabel` the wrapper
                    // with the item as the first argument which the wrapper will
                    // materialize into a local slot matching `iterator`.
                    let ws_name = if let crate::ast::AstNodeKind::Workspace { name, .. } = workspace_node.get_kind() { name.clone() } else { "<anon_ws>".to_string() };
                    let loop_fn_name = format!("{}_forin_{}", ws_name, ir_mod.len());
                    let loop_fn_id = ir_mod.declare_function(&loop_fn_name);
                    let label_idx = (loop_fn_id as usize).saturating_sub(1);
                    // build the wrapper function body
                    let mut fb = super::function_builder::FunctionBuilder::new();
                    // create a local for the iterator name so args[0] seeds it
                    fb.get_or_create_local(iterator);
                    let label_name = format!("L{}", label_idx);
                    fb.emit_op(IROp::Label { name: label_name.clone() });
                    // lower the loop body into the function builder
                    super::lower_stmt::emit_calls_in_node_with_builder(body, &mut fb, ir_mod, ctx);
                    fb.finalize_into(ir_mod);

                    // Now call the wrapper from the loop body with the item as arg
                    let dest = ir_mod.alloc_reg();
                    ir_mod.emit_op(IROp::CallLabel { dest, label_index: label_idx, args: vec![item_reg] });

                    // increment idx: idx = idx + 1
                    let one_reg = ir_mod.alloc_reg();
                    ir_mod.emit_op(IROp::LConst { dest: one_reg, value: crate::ir::value::Value::Int(1) });
                    ir_mod.emit_op(IROp::Add { dest: idx_reg, src1: idx_reg, src2: one_reg });

                    // jump back to condition
                    ir_mod.emit_op(IROp::Jump { target: loop_cond_pos });

                    // emit a label at the end of the loop body so the earlier
                    // placeholder can be resolved to this exact op index later.
                    ir_mod.emit_op(IROp::Label { name: after_label.clone() });
                    // done with the temporary iterator binding
                    ctx.unbind_temp_ident(iterator);
                    continue;
                }
            }
        }

        // Default: lower the statement normally to preserve side-effects
        super::lower_stmt::lower_statment(stmt, ir_mod, ctx);
    }

    // If analyzer marked this workspace as an entrypoint, emit a labeled
    // function to run the workspace body at program start.
    // Analyzer information is available via the context if it was created
    // from AnalyzerOutput (we detect entrypoints by presence of a function
    // with the same node id in the functions map). For now, treat any
    // workspace that has an associated function id as an entrypoint to
    // be invoked at startup.
    if let Some(fid) = ctx.get_function_id(workspace_node.get_id()) {
        // Create a per-workspace function body and emit it into the module.
        let mut fb = super::function_builder::FunctionBuilder::new();
        let label_idx = (fid as usize).saturating_sub(1);
        let label_name = format!("L{}", label_idx);
        fb.emit_op(IROp::Label { name: label_name });
        super::lower_stmt::emit_calls_in_node_with_builder(body, &mut fb, ir_mod, ctx);
        // Ensure the entrypoint function terminates the program cleanly.
        // Without this, control could fall through into subsequent module
        // code and produce unintended execution paths. Emitting a Halt
        // here stops the VM once the entrypoint completes.
        fb.emit_op(IROp::Halt);
        fb.finalize_into(ir_mod);
        // Emit a module-level call to the workspace entrypoint so it runs
        // at program startup. The call targets the function label index we
        // declared above and uses no arguments.
        let call_dest = ir_mod.alloc_reg();
        ir_mod.emit_op(IROp::CallLabel { dest: call_dest, label_index: label_idx, args: vec![] });
    } else {
        // no entrypoint function id; nothing to emit
    }
}

fn lower_project_object(
    project_node: &crate::ast::AstNode,
    ir_mod: &mut crate::ir::module::IrModule,
    ctx: &mut super::lowering_context::LoweringContext,
) {
    // Register the project as an object and lower its member assignments
    if let crate::ast::AstNodeKind::Project { name, body } = project_node.get_kind() {
        // ensure object id exists
        if ctx.get_object_id(project_node.get_id()).is_none() {
            let oid = ir_mod.declare_object(name);
            ctx.bind_object_id(project_node.get_id(), oid);
            ctx.symbols.insert(name.clone(), oid);
        }

        // Create a module-level object runtime slot (a register holding the object)
        let obj_reg = ir_mod.alloc_reg();
        // initialize to an empty object so SetProp writes into a real object
        let empty_map: std::collections::HashMap<String, crate::ir::value::Value> = std::collections::HashMap::new();
        ir_mod.emit_op(IROp::LConst { dest: obj_reg, value: crate::ir::value::Value::Object(empty_map) });
        // record runtime register in lowering context for other passes
        ctx.bind_object_reg(project_node.get_id(), obj_reg);
        // Also bind by declared object id so lookups via symbol->object id
        // mapping can find the runtime register during Member lowering.
        if let Some(obj_id) = ctx.get_object_id(project_node.get_id()) {
            ctx.bind_object_reg_by_objid(obj_id, obj_reg);
        }

        // Lower each statement in the project body; treat assignments to
        // identifiers as setting properties on this object.
        if let crate::ast::AstNodeKind::Block { statements } = body.get_kind() {
            for stmt in statements.iter() {
                if let crate::ast::AstNodeKind::Assignment { target, value } = stmt.get_kind() {
                    if let crate::ast::AstNodeKind::Identifier { name: prop_name } = target.get_kind() {
                        // evaluate value into a register (use the builder-aware helper
                        // so list literals and other expressions are handled)
                        let val_reg = super::lower_expr::lower_expr_to_reg_with_builder(value, ir_mod, ctx, None);
                        // emit key symbol const
                        let key_reg = ir_mod.alloc_reg();
                        ir_mod.emit_op(IROp::LConst { dest: key_reg, value: crate::ir::value::Value::Symbol(prop_name.clone()) });
                        // emit SetProp obj.prop = val
                        ir_mod.emit_op(IROp::SetProp { obj: obj_reg, key: key_reg, src: val_reg });
                        continue;
                    }
                }
                // fallback: lower the statement normally to preserve side-effects
                super::lower_stmt::lower_statment(stmt, ir_mod, ctx);
            }
        }
    }
}

fn lower_stage_object(
    _stage_node: &crate::ast::AstNode,
    _ir_mod: &mut crate::ir::module::IrModule,
    _ctx: &mut super::lowering_context::LoweringContext,
) {
    // Register the stage as a function prototype so calls can reference it.
    if let crate::ast::AstNodeKind::Stage { name, .. } = _stage_node.get_kind() {
        // If not already declared, declare and bind
        if _ctx.get_function_id(_stage_node.get_id()).is_none() {
            let id = _ir_mod.declare_function(name);
            _ctx.bind_function_id(_stage_node.get_id(), id);
            // Also populate the symbol map for name->id lookups
            _ctx.symbols.insert(name.clone(), id);
        }
    }
}

/// Collect member definitions from a container body node.
fn collect_member_definitions(
    object_node: &crate::ast::AstNode,
    _ir_mod: &mut crate::ir::module::IrModule,
    _ctx: &mut super::lowering_context::LoweringContext,
) -> Vec<crate::ast::AstNode> {
    let mut members: Vec<crate::ast::AstNode> = Vec::new();
    if let crate::ast::AstNodeKind::Block { statements } = object_node.get_kind() {
        for stmt in statements {
            match stmt.get_kind() {
                // Collect project declarations and simple assignments so we can
                // detect static list initializers (e.g. `projects = [p]`) and
                // lower iteration in a later pass. Also collect `ForIn`
                // so workspace-level loops are handled after array constants
                // have been emitted. Other statements are lowered immediately
                // to preserve side-effects.
                crate::ast::AstNodeKind::Project { .. } => { members.push(stmt.clone()); }
                crate::ast::AstNodeKind::Assignment { .. } => { members.push(stmt.clone()); }
                crate::ast::AstNodeKind::ForIn { .. } => { members.push(stmt.clone()); }
                _ => {
                    // Preserve side-effecting top-level statements by collecting
                    // them for the second pass. We previously lowered these
                    // eagerly which could cause bodies (e.g. ForIn inner
                    // statements) to be emitted at module scope before wrapper
                    // functions were created. Collecting here lets the
                    // workspace lowering decide how to lower each member in
                    // the correct context (module vs wrapper).
                    members.push(stmt.clone());
                }
            }
        }
    }
    members
}