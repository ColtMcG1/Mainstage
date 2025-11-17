use crate::semantic::{SymbolTable, SymbolKind};
use crate::parser::{AstNode, AstType};
use crate::semantic::analyzer::SemanticAnalyzer;

pub(crate) fn is_kind<'a>(table: &SymbolTable<'a>, name: &str, kind: SymbolKind) -> bool {
    if let Some(syms) = table.get(name) {
        syms.iter().any(|s| s.kind() == &kind)
    } else { false }
}

pub(crate) fn mark_kind<'a>(table: &mut SymbolTable<'a>, name: &str, kind: SymbolKind) {
    if let Some(syms) = table.get_mut(name) {
        for s in syms {
            if s.kind() == &kind { s.increment_reference_count(); }
        }
    }
}

// Resolve id.member generically across scopes.
// Validates: container exists, member exists for that container, and container has been initialized in current frame(s).
pub(crate) fn handle_scoped_member<'a>(
    an: &mut SemanticAnalyzer<'a>,
    target: &AstNode<'a>,
    member: &AstNode<'a>,
) -> Result<(), ()> {
    let (container, member_name) = match (&target.kind, &member.kind) {
        (AstType::Identifier { name: c }, AstType::Identifier { name: m }) => (c.as_ref(), m.as_ref()),
        _ => return Ok(()), // dynamic member or complex target: defer
    };

    // Determine container kind
    let kind = if is_kind(&an.symbol_table, container, SymbolKind::Workspace) {
        SymbolKind::Workspace
    } else if is_kind(&an.symbol_table, container, SymbolKind::Project) {
        SymbolKind::Project
    } else if is_kind(&an.symbol_table, container, SymbolKind::Stage) {
        SymbolKind::Stage
    } else if is_kind(&an.symbol_table, container, SymbolKind::Task) {
        SymbolKind::Task
    } else {
        crate::report!(
            crate::reports::Level::Error,
            format!("Unknown container '{}'.", container),
            Some("SemanticAnalyzer".into()),
            target.span.clone(),
            target.location.clone()
        );
        return Err(());
    };

    // Member existence
    let key = (kind.clone(), container.to_string());
    let exists = an.scope_members.get(&key).map(|m| m.contains_key(member_name)).unwrap_or(false);
    if !exists {
        crate::report!(
            crate::reports::Level::Error,
            format!("Unknown member '{}' on {} '{}'.", member_name, format!("{:?}", kind).to_lowercase(), container),
            Some("SemanticAnalyzer".into()),
            member.span.clone(),
            member.location.clone()
        );
        return Err(());
    }

    // Initialization check
    if !an.is_scope_initialized(kind.clone(), container) {
        crate::report!(
            crate::reports::Level::Error,
            format!("Cannot access member '{}' before {} '{}' is run.", member_name, format!("{:?}", kind).to_lowercase(), container),
            Some("SemanticAnalyzer".into()),
            member.span.clone(),
            member.location.clone()
        );
        return Err(());
    }

    // Mark container as referenced
    if let Some(vec) = an.symbol_table.get_mut(container) {
        for s in vec { s.increment_reference_count(); }
    }
    Ok(())
}

pub(crate) fn handle_identifier<'a>(
    analyzer: &mut crate::semantic::analyzer::SemanticAnalyzer<'a>,
    node: &crate::parser::AstNode<'a>,
    name: &str,
) -> Result<(), ()> {
    // Example implementation: increment reference count if symbol exists
    if let Some(symbols) = analyzer.symbol_table.get_mut(name) {
        for symbol in symbols {
            symbol.increment_reference_count();
        }
        Ok(())
    } else {
        crate::report!(
            crate::reports::Level::Error,
            format!("Undefined identifier '{}'.", name),
            Some("SemanticAnalyzer".into()),
            node.span.clone(),
            node.location.clone()
        );
        Err(())
    }
}