use crate::ast::AstNode;
use crate::error::{MainstageErrorExt};
use crate::analyzers::output::{AnalyzerOutput, NodeId};

mod err;
mod kind;
mod stmt;
mod expr;
mod node;
mod symbol;
pub mod table;
mod analyzer;

pub use kind::InferredKind;

pub fn analyze_semantic_rules(ast: &mut AstNode) -> Result<(String, AnalyzerOutput), Vec<Box<dyn MainstageErrorExt>>> {
    let mut analyzer = analyzer::Analyzer::new();

    // Run the analyzer and propagate any fatal analysis error immediately.
    if let Err(e) = analyzer.analyze(ast) {
        return Err(vec![e]);
    }

    // Collect diagnostics (warnings/infos) produced by the analyzer. If any
    // diagnostics were collected, return them as the error path so callers can
    // display or handle them.
    let diagnostics = analyzer.take_diagnostics();
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    // The symbol table holds the chosen entrypoint workspace name (if any).
    // Return it to the caller as a String. If no workspace was found, return
    // a diagnostic error.
    let entrypoint = analyzer.get_symbol_table().entrypoint();
    if let Some(node) = entrypoint {
        // Build a minimal AnalyzerOutput from the symbol table so lowering can
        // consume resolved symbols without re-traversing the AST. For now we
        // produce object and function entries with analyzer-local NodeIds.
        let mut analysis = AnalyzerOutput::new();

        // Assign incremental node ids for each discovered symbol
        let mut next_node_id: NodeId = 1;

        // Build initial function/object entries from the symbol table (visible symbols)
        use std::collections::HashMap;
        let mut func_name_to_node: HashMap<String, NodeId> = HashMap::new();
        // First pass: create function/object entries and build a name->node map
        for scope in &analyzer.get_symbol_table().symbols {
            for (name, syms) in scope.iter() {
                if let Some(sym) = syms.last() {
                    match sym.kind() {
                        crate::analyzers::semantic::symbol::SymbolKind::Object => {
                            analysis.objects.push(crate::analyzers::output::ObjectInfo {
                                node_id: next_node_id,
                                name: name.clone(),
                                span: sym.span().clone(),
                                members: Vec::new(),
                                parent: None,
                            });
                            next_node_id += 1;
                        }
                        crate::analyzers::semantic::symbol::SymbolKind::Function => {
                            analysis.functions.push(crate::analyzers::output::FunctionInfo {
                                node_id: next_node_id,
                                name: Some(name.clone()),
                                span: sym.span().clone(),
                                params: Vec::new(),
                                return_type: sym.returns().cloned(),
                                prototype_id: None,
                                captures: Vec::new(),
                            });
                            func_name_to_node.insert(name.clone(), next_node_id);
                            next_node_id += 1;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Second pass: build scope info with symbol nodes
        for scope in &analyzer.get_symbol_table().symbols {
            let scope_node = next_node_id;
            next_node_id += 1;
            let mut symbols = Vec::new();
            for (name, syms) in scope.iter() {
                if let Some(sym) = syms.last() {
                    // find an existing node id for functions/objects, or allocate a new one
                    let node_id = if let Some(&nid) = func_name_to_node.get(name) {
                        nid
                    } else {
                        let nid = next_node_id; next_node_id += 1; nid
                    };
                    let kind = match sym.kind() {
                        crate::analyzers::semantic::symbol::SymbolKind::Object => crate::analyzers::output::SymbolKind::Object,
                        crate::analyzers::semantic::symbol::SymbolKind::Function => crate::analyzers::output::SymbolKind::Function,
                        crate::analyzers::semantic::symbol::SymbolKind::Variable => crate::analyzers::output::SymbolKind::Variable,
                    };
                    symbols.push(crate::analyzers::output::SymbolInfo {
                        name: name.clone(),
                        kind,
                        node_id,
                        span: sym.span().clone(),
                        ty: sym.inferred_type().cloned(),
                        usages: sym.usages.clone(),
                    });
                }
            }
            analysis.scopes.push(crate::analyzers::output::ScopeInfo { node_id: scope_node, parent: None, symbols });
        }

        // Traverse AST to fill function params and call graph.
        fn collect_from_node(
            node: &crate::ast::AstNode,
            current_func: Option<NodeId>,
            analysis: &mut AnalyzerOutput,
            func_name_to_node: &HashMap<String, NodeId>,
        ) {
            use crate::ast::AstNodeKind;

            match node.get_kind() {
                AstNodeKind::Stage { name, args, body } => {
                    // Prefer to locate the function info by name and update its
                    // node id/span if we discover the AST node for it.
                    if let Some(fi) = analysis.functions.iter_mut().find(|f| f.name.as_deref() == Some(name.as_str())) {
                        // Update node id and span to the AST node
                        let nid = node.get_id();
                        fi.node_id = nid;
                        fi.span = node.get_span().cloned();

                        // collect params from AST
                        if let Some(args_node) = args.as_ref() {
                            if let AstNodeKind::Arguments { args: param_nodes } = args_node.get_kind() {
                                let mut params = Vec::new();
                                for p in param_nodes {
                                    if let AstNodeKind::Identifier { name: pname } = p.get_kind() {
                                        params.push(crate::analyzers::output::ParamInfo {
                                            name: pname.clone(),
                                            span: p.get_span().cloned(),
                                            ty: None,
                                        });
                                    }
                                }
                                fi.params = params;
                            }
                        }

                        // record mapping from name -> node id for call graph collection
                        // (overwrite any earlier synthetic node id)
                        // Note: we don't need to update func_name_to_node map here for
                        // the outer scope since we use function name lookups by name.

                        // traverse body with current function = nid
                        collect_from_node(body, Some(nid), analysis, func_name_to_node);
                    }
                }
                AstNodeKind::Call { callee, args } => {
                    // If callee is an identifier and resolves to a known function, add edge
                    if let AstNodeKind::Identifier { name } = callee.get_kind() {
                        if let Some(target_f) = analysis.functions.iter().find(|f| f.name.as_deref() == Some(name.as_str())) {
                            let target_nid = target_f.node_id;
                            if let Some(src) = current_func {
                                analysis.call_graph.push((src, target_nid));
                            }
                        }
                    }
                    // traverse args
                    for a in args {
                        collect_from_node(a, current_func, analysis, func_name_to_node);
                    }
                }
                AstNodeKind::Script { body } => {
                    for b in body {
                        collect_from_node(b, current_func, analysis, func_name_to_node);
                    }
                }
                AstNodeKind::Block { statements } => {
                    for s in statements {
                        collect_from_node(s, current_func, analysis, func_name_to_node);
                    }
                }
                AstNodeKind::If { condition, body } => {
                    collect_from_node(condition, current_func, analysis, func_name_to_node);
                    collect_from_node(body, current_func, analysis, func_name_to_node);
                }
                AstNodeKind::IfElse { condition, if_body, else_body } => {
                    collect_from_node(condition, current_func, analysis, func_name_to_node);
                    collect_from_node(if_body, current_func, analysis, func_name_to_node);
                    collect_from_node(else_body, current_func, analysis, func_name_to_node);
                }
                AstNodeKind::ForIn { iterable, body, .. } => {
                    collect_from_node(iterable, current_func, analysis, func_name_to_node);
                    collect_from_node(body, current_func, analysis, func_name_to_node);
                }
                AstNodeKind::ForTo { initializer, limit, body } => {
                    collect_from_node(initializer, current_func, analysis, func_name_to_node);
                    collect_from_node(limit, current_func, analysis, func_name_to_node);
                    collect_from_node(body, current_func, analysis, func_name_to_node);
                }
                AstNodeKind::While { condition, body } => {
                    collect_from_node(condition, current_func, analysis, func_name_to_node);
                    collect_from_node(body, current_func, analysis, func_name_to_node);
                }
                AstNodeKind::UnaryOp { expr, .. } => {
                    collect_from_node(expr, current_func, analysis, func_name_to_node);
                }
                AstNodeKind::BinaryOp { left, right, .. } => {
                    collect_from_node(left, current_func, analysis, func_name_to_node);
                    collect_from_node(right, current_func, analysis, func_name_to_node);
                }
                AstNodeKind::Assignment { target, value } => {
                    collect_from_node(target, current_func, analysis, func_name_to_node);
                    collect_from_node(value, current_func, analysis, func_name_to_node);
                }
                _ => {}
            }
        }

        // Start traversal from script root
        collect_from_node(ast, None, &mut analysis, &func_name_to_node);

        Ok((node, analysis))
    } else {
        Err(vec![Box::new(
            err::SemanticError::with(
                crate::error::Level::Error,
                "No entrypoint workspace found in script.".to_string(),
                "mainstage.analyzers.semantic.analyze_semantic_rules".to_string(),
                ast.location.clone(),
                ast.span.clone(),
            ),
        )])
    }
}