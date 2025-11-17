use crate::parser::{AstNode, AstType};
use crate::report;
use crate::reports::*;
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

    // Simple identifier calls
    if let AstType::Identifier { name } = &target.kind {
        let name = name.as_ref();

        // builtins
        if an.is_builtin(name) {
            check_builtin_arity(node, name, arguments)?;
            for a in arguments { an.analyze_expression(a)?; }
            if !in_expression && an.is_value_builtin(name) {
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

        // scope init calls: workspace/project/stage/task
        let mut handled_scope_call = false;
        for kind in [SymbolKind::Workspace, SymbolKind::Project, SymbolKind::Stage, SymbolKind::Task] {
            if util::is_kind(&an.symbol_table, name, kind.clone()) {
                an.mark_scope_initialized(kind, name);
                handled_scope_call = true;
                break;
            }
        }
        if handled_scope_call {
            for a in arguments { an.analyze_expression(a)?; }
            if in_expression {
                report!(
                    Level::Error,
                    format!("Scope '{}' call returns no value.", name),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                return Err(());
            }
            return Ok(());
        }

        // tasks by bare name
        if an.is_task_name(name) {
            util::mark_kind(&mut an.symbol_table, name, SymbolKind::Task);
            for a in arguments { an.analyze_expression(a)?; }
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

    // Member call targets: not supported as callable by default; analyze args for errors anyway.
    for a in arguments { an.analyze_expression(a)?; }
    report!(
        Level::Error,
        "Member calls are not supported here.".into(),
        Some("SemanticAnalyzer".into()),
        node.span.clone(),
        node.location.clone()
    );
    Err(())
}

fn check_builtin_arity<'a>(node: &AstNode<'a>, name: &str, args: &[AstNode<'a>]) -> Result<(), ()> {
    let err = |m: &str| {
        report!(Level::Error, m.into(), Some("SemanticAnalyzer".into()), node.span.clone(), node.location.clone());
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
