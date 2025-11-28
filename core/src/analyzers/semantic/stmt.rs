use super::{
    symbol::{SymbolKind, SymbolScope},
    table::SymbolTable,
};
use crate::analyzers::semantic::symbol::Symbol;
use crate::ast::{AstNode, AstNodeKind};
use crate::error::{Level, MainstageErrorExt};

pub(crate) fn analyze_script_statements(
    node: &mut AstNode,
    tbl: &mut SymbolTable,
) -> Result<(), Box<dyn MainstageErrorExt>> {
    let script_body = match &mut node.kind {
        crate::ast::AstNodeKind::Script { body } => body,
        _ => {
            return Err(Box::new(
                crate::analyzers::semantic::err::SemanticError::with(
                    Level::Error,
                    "Expected a Script node.".to_string(),
                    "mainstage.analyzers.semantic.stmt.analyze_script_statements".to_string(),
                    node.location.clone(),
                    node.span.clone(),
                ),
            ));
        }
    };

    for statement in script_body.iter_mut() {
        analyze_statement(statement, tbl)?;
    }

    Ok(())
}

fn analyze_statement(
    node: &mut AstNode,
    tbl: &mut SymbolTable,
) -> Result<(), Box<dyn MainstageErrorExt>> {
    match &mut node.kind {
        AstNodeKind::Workspace { name, body } => {

            // ensure body is not empty
            check_for_empty_block(body)?;

            // register workspace in current (global) scope
            tbl.insert_symbol(Symbol::new_object(
                name.clone(),
                SymbolScope::Global,
                None,
                None,
                node.location.clone(),
                node.span.clone(),
            ));

            // analyze children inside a new object declaration scope (workspace)
            tbl.enter_object_scope(name.clone());
            super::node::analyze_node(body, tbl)?;
            tbl.exit_scope();
        }
        AstNodeKind::Project { name, body } => {

            // ensure body is not empty
            check_for_empty_block(body)?;

            // register project in current (global) scope
            tbl.insert_symbol(Symbol::new_object(
                name.clone(),
                SymbolScope::Global,
                None,
                None,
                node.location.clone(),
                node.span.clone(),
            ));

            // analyze children inside a new object declaration scope (project)
            tbl.enter_object_scope(name.clone());
            super::node::analyze_node(body, tbl)?;
            tbl.exit_scope();
        }

        AstNodeKind::Stage { name, args, body } => {

            // ensure body is not empty
            check_for_empty_block(body)?;
            
            // Build parameter symbol list (do not insert yet)
            let params_symbols = if let Some(params_node) = args {
                analyze_parameters(params_node, tbl)?
            } else {
                Vec::new()
            };

            // Insert stage symbol into global scope with parameter metadata
            tbl.insert_symbol(Symbol::new(
                name.clone(),
                SymbolKind::Function,
                None,
                SymbolScope::Global,
                Some(params_symbols.clone()),
                None,
                node.location.clone(),
                node.span.clone(),
            ));

            // Enter stage-local scope and insert parameter symbols for use inside the body
            tbl.enter_scope();
            for p in params_symbols.iter() {
                tbl.insert_symbol(p.clone());
            }
            super::node::analyze_node(body, tbl)?;

            // Collect returns from the stage body before exiting the scope so return expressions
            // can be analyzed with access to local symbols/params.
            if let Some(returns_kind) = super::expr::collect_returns(body, tbl)? {
                // set the stage symbol's returns metadata in the global scope
                if let Some(sym) = tbl.get_latest_symbol_mut(name) {
                    // ensure we are updating the global-stage symbol specifically
                    // if the found symbol is in the current scope this will still update
                    // the most-recent visible symbol; for strict global-only update,
                    // use a dedicated get_symbol_in_global_scope_mut helper instead.
                    sym.set_returns(returns_kind);
                }
            }

            tbl.exit_scope();
        }
        AstNodeKind::Null => {
            // EOI emits a Null node, do nothing
        }
        _ => {
            return Err(Box::new(
                crate::analyzers::semantic::err::SemanticError::with(
                    Level::Error,
                    format!(
                        "Unsupported statement type in script body: {}\nSupport types are objects such as Workspace, Project, and Stage.",
                        node.kind
                    ),
                    "mainstage.analyzers.semantic.stmt.analyze_statement".to_string(),
                    node.location.clone(),
                    node.span.clone(),
                ),
            ));
        }
    }
    Ok(())
}

fn check_for_empty_block(
    block_node: &AstNode,
) -> Result<(), Box<dyn MainstageErrorExt>> {
    if let AstNodeKind::Block { statements } = &block_node.kind {
        if statements.is_empty() {
            return Err(Box::new(
                crate::analyzers::semantic::err::SemanticError::with(
                    Level::Error,
                    "Block cannot be empty.".to_string(),
                    "mainstage.analyzers.semantic.stmt.check_for_empty_block".to_string(),
                    block_node.location.clone(),
                    block_node.span.clone(),
                ),
            ));
        }
    }
    Ok(())
}

fn analyze_parameters(
    args: &mut AstNode,
    _tbl: &mut SymbolTable,
) -> Result<Vec<Symbol>, Box<dyn MainstageErrorExt>> {
    let mut params_symbols = Vec::new();

    if let AstNodeKind::Arguments { args } = &mut args.kind {
        for param in args.iter_mut() {
            if let AstNodeKind::Identifier { name } = &mut param.kind {
                let symbol = Symbol::new(
                    name.clone(),
                    SymbolKind::Variable,
                    None,
                    SymbolScope::Local,
                    None,
                    None,
                    param.location.clone(),
                    param.span.clone(),
                );
                // do NOT insert into the table here; caller will insert into the correct scope
                params_symbols.push(symbol);
            } else {
                return Err(Box::new(
                    crate::analyzers::semantic::err::SemanticError::with(
                        Level::Error,
                        "Expected Parameter node.".to_string(),
                        "mainstage.analyzers.semantic.stmt.analyze_parameters".to_string(),
                        param.location.clone(),
                        param.span.clone(),
                    ),
                ));
            }
        }
    } else {
        return Err(Box::new(
            crate::analyzers::semantic::err::SemanticError::with(
                Level::Error,
                "Expected Parameters node.".to_string(),
                "mainstage.analyzers.semantic.stmt.analyze_parameters".to_string(),
                args.location.clone(),
                args.span.clone(),
            ),
        ));
    }

    Ok(params_symbols)
}
