//! ./parser/expander.rs
//!
//! Module for include file expansion within scripts.
//! This module provides functionality to handle `#include` directives and expand them into the main script.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

use crate::parser::AstParser;
use crate::report;
use crate::reports::*;
use crate::scripts::*;

use std::rc::Rc;

/// Expands `#include` directives in the given script by replacing them with the content of the included scripts.
/// # Arguments
/// * `ast` - The AST parser containing the script to expand.
/// * `script` - The script to expand.
/// # Returns
/// * `Result<(), ()>` - Ok if expansion is successful, Err otherwise.

pub fn expand_includes(ast: &mut AstParser, script: Rc<script::Script>) -> Result<(), ()> {
    for node in ast.root.children.iter().cloned().collect::<Vec<_>>() {
        if let crate::parser::node::AstType::Include { path } = &node.kind {
            // Process the include directive
            if let Err(e) = process_include(&node, path, ast, script.clone()) {
                report!(
                    Level::Error,
                    e,
                    Some("mainstage.expander".into()),
                    None,
                    None
                );
                return Err(());
            }
        }
    }
    Ok(())
}

fn process_include(
    node: &crate::parser::node::AstNode,
    path: &str,
    ast: &mut AstParser,
    script: Rc<script::Script>,
) -> Result<(), String> {
    // Resolve the include path
    let include_path = resolve_include_path(path, &script)?;

    // Read the include file
    let include_content = read_include_file(&include_path)?;

    // Create a new script instance for the included file
    let include_script = Rc::new(script::Script::new(
        include_path.to_string_lossy().to_string(),
        include_path.clone(),
        include_content,
    ));

    // Parse the included content into an AST
    let mut include_ast = parse_include_ast(&include_script)?;

    // Recursively expand includes in the included AST
    let _ = expand_includes(&mut include_ast, include_script);

    // Replace the include node with the children of the included AST
    replace_include_node(ast, node, include_ast.root.children);

    Ok(())
}

fn resolve_include_path(path: &str, script: &script::Script) -> Result<std::path::PathBuf, String> {
    let path = std::path::Path::new(path);

    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    script
        .path
        .parent()
        .map(|base| base.join(path))
        .ok_or_else(|| format!("Failed to resolve include path: {}", path.display()))
}

fn read_include_file(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|_| format!("Failed to read include file: {}", path.display()))
}

fn parse_include_ast(script: &script::Script) -> Result<AstParser, String> {
    AstParser::new(script)
        .map_err(|_| format!("Failed to parse include file: {}", script.path.display()))
}

fn replace_include_node(
    ast: &mut AstParser,
    include_node: &crate::parser::node::AstNode,
    new_children: Vec<crate::parser::node::AstNode>,
) {
    if let Some(pos) = ast.root.children.iter().position(|n| n == include_node) {
        // Remove the include node
        ast.root.children.remove(pos);

        // Insert the new children
        for (i, child) in new_children
            .into_iter()
            .map(|child| child.into_owned())
            .enumerate()
        {
            ast.root.children.insert(pos + i, child);
        }
    }
}
