use crate::parser::{AstNode, AstType};
use crate::report;
use crate::reports::*;
use crate::semantic::types::InferredType;
use crate::semantic::SymbolKind;
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::analyzer::util;

pub(crate) fn analyze_call<'a>(
    an: &mut SemanticAnalyzer<'a>,
    node: &AstNode<'a>,
    in_expression: bool,
) -> Result<(), ()> {
    let (target, arguments): (&AstNode<'a>, &[AstNode<'a>]) = match &node.kind {
        AstType::Call { target, arguments } => (target, arguments),
        _ => return Ok(()),
    };

    // 1) Identifier targets (functions / scopes)
    if let AstType::Identifier { name } = &target.kind {
        let name = name.as_ref();

        if an.is_builtin_function(name) {
            check_builtin_arity(node, name, arguments)?;
            for a in arguments {
                an.analyze_expression(a)?;
            }
            if !in_expression
                && an
                    .get_builtin_function(name)
                    .map_or(false, |f| f.returns != InferredType::Unit)
            {
                report!(
                    Level::Warning,
                    format!("Return value of builtin '{}' discarded.", name),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
            }
            return Ok(());
        }

        // Any scope kind can be callable; both args and returns supported for stages & tasks.
        for kind in [
            SymbolKind::Workspace,
            SymbolKind::Project,
            SymbolKind::Stage,
            SymbolKind::Task,
        ] {
            if util::is_kind(&an.symbol_table, name, kind.clone()) {
                an.symbol_table.bump_refs(name); // NEW: count reference
                an.mark_scope_initialized(kind.clone(), name);
                util::mark_kind(&mut an.symbol_table, name, kind.clone());
                for a in arguments { an.analyze_expression(a)?; }
                let returns_value = scope_has_value_return(an, &kind, name);
                if in_expression {
                    if returns_value { return Ok(()); }
                    report!(Level::Error,
                        format!("Scope '{}' call returns no value.", name),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(), node.location.clone());
                    return Err(());
                } else {
                    if returns_value {
                        report!(Level::Warning,
                            format!("Return value of scope '{}' discarded.", name),
                            Some("SemanticAnalyzer".into()),
                            node.span.clone(), node.location.clone());
                    }
                    return Ok(());
                }
            }
        }

        if an.is_task_name(name) {
            util::mark_kind(&mut an.symbol_table, name, SymbolKind::Task);
            for a in arguments {
                an.analyze_expression(a)?;
            }
            let returns_value = an.task_returns.get(name).is_some();
            if !in_expression && returns_value {
                report!(
                    Level::Warning,
                    format!("Return value of task '{}' discarded.", name),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
            }
            return Ok(());
        }

        report!(
            Level::Error,
            format!("Unknown callable '{}'", name),
            Some("SemanticAnalyzer".into()),
            node.span.clone(),
            node.location.clone()
        );
        return Err(());
    }

    // 2) Member call targets: container.member(args)
    if let AstType::Member { target: recv, member } = &target.kind {
        // Analyze receiver first
        an.analyze_expression(recv)?;

        // Builtin method handling (uses is_builtin_method/get_builtin_method, clears dead_code warnings)
        if let AstType::Identifier { name: mname } = &member.kind {
            let m = mname.as_ref();
            if an.is_builtin_method(m) {
                if let Some(def) = an.get_builtin_method(m) {
                    let def_name = def.name;
                    let def_arity = def.arity;
                    let def_variadic = def.variadic;
                    let def_returns = def.returns;
                    if !def_variadic && arguments.len() != def_arity {
                        report!(
                            Level::Error,
                            format!("Builtin method '{}' expects {} arg(s).", def_name, def_arity),
                            Some("SemanticAnalyzer".into()),
                            node.span.clone(),
                            node.location.clone()
                        );
                        return Err(());
                    }
                    for a in arguments { an.analyze_expression(a)?; }
                    if !in_expression && def_returns != InferredType::Unit {
                        report!(
                            Level::Warning,
                            format!("Return value of method '{}' discarded.", def_name),
                            Some("SemanticAnalyzer".into()),
                            node.span.clone(),
                            node.location.clone()
                        );
                    }
                    return Ok(());
                }
            }
        }

        let (container_name, member_name) = match (&recv.kind, &member.kind) {
            (AstType::Identifier { name: c }, AstType::Identifier { name: m }) => (c.as_ref(), m.as_ref()),
            _ => {
                for a in arguments { an.analyze_expression(a)?; }
                return Ok(());
            }
        };

        // Resolve container kind
        let container_kind =
            if util::is_kind(&an.symbol_table, container_name, SymbolKind::Workspace) {
                Some(SymbolKind::Workspace)
            } else if util::is_kind(&an.symbol_table, container_name, SymbolKind::Project) {
                Some(SymbolKind::Project)
            } else if util::is_kind(&an.symbol_table, container_name, SymbolKind::Stage) {
                Some(SymbolKind::Stage)
            } else if util::is_kind(&an.symbol_table, container_name, SymbolKind::Task) {
                Some(SymbolKind::Task)
            } else {
                None
            };

        // Resolve member scope kind by name (stage/task/project/workspace)
        let mut member_kind: Option<SymbolKind> = None;
        for k in [SymbolKind::Stage, SymbolKind::Task, SymbolKind::Project, SymbolKind::Workspace] {
            if find_scope_node(an, &k, member_name).is_some() {
                member_kind = Some(k);
                break;
            }
        }

        if let Some(ck) = container_kind {
            if let Some(mk) = member_kind {
                an.mark_scope_initialized(mk, member_name);
                util::mark_kind(&mut an.symbol_table, container_name, ck);
                util::mark_kind(&mut an.symbol_table, member_name, mk);
                for a in arguments { an.analyze_expression(a)?; }

                let returns_value = scope_has_value_return(an, &mk, member_name);
                if in_expression {
                    if returns_value { return Ok(()); }
                    report!(
                        Level::Error,
                        format!("'{}.{}' call returns no value.", container_name, member_name),
                        Some("SemanticAnalyzer".into()),
                        node.span.clone(),
                        node.location.clone()
                    );
                    return Err(());
                } else {
                    if returns_value {
                        report!(
                            Level::Warning,
                            format!("Return value of '{}.{}' discarded.", container_name, member_name),
                            Some("SemanticAnalyzer".into()),
                            node.span.clone(),
                            node.location.clone()
                        );
                    }
                    return Ok(());
                }
            }

            // No resolvable callable member by that name
            report!(
                Level::Error,
                format!("Unknown callable '{}.{}'", container_name, member_name),
                Some("SemanticAnalyzer".into()),
                node.span.clone(),
                node.location.clone()
            );
            for a in arguments { an.analyze_expression(a)?; }
            return Err(());
        }

        // Unknown container
        report!(
            Level::Error,
            format!("Unknown callable '{}.{}'", container_name, member_name),
            Some("SemanticAnalyzer".into()),
            node.span.clone(),
            node.location.clone()
        );
        for a in arguments { an.analyze_expression(a)?; }
        return Err(());
    }

    // 3) Fallback: analyze args to continue semantic pass
    for a in arguments {
        an.analyze_expression(a)?;
    }
    Ok(())
}

// Walk the AST for the named scope and check if any Return has a value.
fn scope_has_value_return<'a>(an: &SemanticAnalyzer<'a>, kind: &SymbolKind, name: &str) -> bool {
    if let Some(scope_node) = find_scope_node(an, kind, name) {
        return has_value_return(scope_node);
    }
    false
}

fn find_scope_node<'a>(
    an: &'a SemanticAnalyzer<'a>,
    kind: &SymbolKind,
    name: &str,
) -> Option<&'a AstNode<'a>> {
    fn matches(node: &AstNode<'_>, kind: &SymbolKind, name: &str) -> bool {
        match (kind, &node.kind) {
            (SymbolKind::Workspace, AstType::Workspace { name: n }) => n.as_ref() == name,
            (SymbolKind::Project, AstType::Project { name: n }) => n.as_ref() == name,
            (SymbolKind::Stage, AstType::Stage { name: n, .. }) => n.as_ref() == name,
            (SymbolKind::Task, AstType::Task { name: n, .. }) => n.as_ref() == name,
            _ => false,
        }
    }

    let mut stack: Vec<&AstNode<'a>> = vec![an.ast.root()];
    while let Some(n) = stack.pop() {
        if matches(n, kind, name) {
            return Some(n);
        }
        for c in &n.children {
            stack.push(c);
        }
    }
    None
}

fn has_value_return<'a>(node: &'a AstNode<'a>) -> bool {
    if let AstType::Return = node.kind {
        return node.children.get(0).is_some();
    }
    for c in &node.children {
        if has_value_return(c) {
            return true;
        }
    }
    false
}

fn check_builtin_arity<'a>(node: &AstNode<'a>, name: &str, args: &[AstNode<'a>]) -> Result<(), ()> {
    let err = |m: &str| {
        report!(
            Level::Error,
            m.into(),
            Some("SemanticAnalyzer".into()),
            node.span.clone(),
            node.location.clone()
        );
        Err(())
    };
    match name {
        "say" if args.len() != 1 => err("say expects 1 argument"),
        "ask" if args.len() > 1 => err("ask expects 0 or 1 argument"),
        "read" if args.len() != 1 => err("read expects 1 argument"),
        "write" if args.len() != 2 => err("write expects 2 arguments"),
        _ => Ok(()),
    }
}
