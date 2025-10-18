//! ./parser/node.rs
//!
//! Module for handling the Abstract Syntax Tree (AST) nodes and parsing logic.
//! This module provides the `AstNode` struct and related functionality.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18

pub use crate::parser::types::AstType;
use crate::report;
use crate::reports::*;
use crate::scripts::script::Script;

use std::borrow::Cow;

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"] // Path to the grammar file
pub struct MainstageParser;

/// Represents a node in the Abstract Syntax Tree (AST).
/// Each node has a type and may have child nodes.
/// # Examples
/// ```
/// use mainstage::parser::{Node, Type};
///
/// let node = Node::new(Type::Script, vec![]);
/// ```
#[derive(Debug, Clone)]
pub struct AstNode<'a> {
    /// The type of the AST node.
    pub kind: AstType<'a>,
    /// The span of the AST node in the source script.
    pub span: Option<locations::Span>,
    /// The location of the AST node in the source script.
    pub location: Option<locations::Location<'a>>,
    /// The child nodes of the AST node.
    pub children: Vec<AstNode<'a>>,
}

impl<'a> AstNode<'a> {
    /// Creates a new `AstNode` from the given parsing pairs.
    /// This function processes the pairs and constructs the AST recursively.
    /// # Arguments
    /// * `pairs` - The parsing pairs obtained from the parser.
    /// # Returns
    /// * `Ok(AstNode)` if the AST is successfully created.
    /// * `Err(Report)` if there is an error during AST creation.
    pub fn new(pairs: pest::iterators::Pairs<'a, Rule>, script: &Script) -> Result<Self, ()> {
        if pairs.clone().count() == 0 {
            return Err(());
        } else {
            return Ok(AstNode::process_node(
                pairs.into_iter().next().unwrap(),
                script,
            ));
        }
    }

    /// Converts a `pest::Span` to a `locations::Span`.
    /// # Arguments
    /// * `span` - The `pest::Span` to convert.
    /// # Returns
    /// * A `locations::Span` representing the converted span.
    fn convert_pest_span_to_span(span: pest::Span) -> locations::Span {
        locations::Span::new(span.start(), span.end())
    }

    /// Converts a `pest::Span` to a `locations::Location`.
    /// # Arguments
    /// * `span` - The `pest::Span` to convert.
    /// * `script` - The script being processed.
    /// # Returns
    /// * A `locations::Location` representing the converted location.
    fn convert_pest_span_to_location(
        span: pest::Span,
        script: &Script,
    ) -> locations::Location<'static> {
        let span = Self::convert_pest_span_to_span(span);
        match &script.location(span.start) {
            Some(loc) => loc
                .clone()
                .with_file(Cow::Owned(script.path().to_string_lossy().into()))
                .into_owned(),
            None => locations::Location {
                file: Cow::Owned(script.path().to_string_lossy().into()),
                line: 0,
                column: 0,
            },
        }
    }

    /// Recursively processes a parsing pair to create an `AstNode`.
    /// This function matches the rule of the pair and constructs the corresponding AST node.
    /// # Arguments
    /// * `pair` - The parsing pair to be processed.
    /// # Returns
    /// * An `AstNode` representing the parsed structure.
    fn process_node(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        // Extract span and location before moving `pair`
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        // Now process based on the rule
        match pair.as_rule() {
            // --- Top-Level Handlers ---
            Rule::script => AstNode::process_script_rule(pair, script),
            Rule::statement => AstNode::process_statement_rule(pair, script),
            // --- Specific Statement Handlers ---
            // Ignore empty lines and end of input
            Rule::EOI => AstNode {
                kind: AstType::Null,
                span: None,
                location: None,
                children: vec![],
            },
            // --- Fallback for Unhandled Rules ---
            _ => {
                // Log a warning for unhandled rules
                report!(
                    Level::Warning,
                    format!("Unhandled rule: {:?}", pair.as_rule()),
                    Some("mainstage.parser.processor.node".to_string()),
                    Some(span.clone()),
                    Some(location.clone())
                );
                // Process children
                let children = pair
                    .into_inner()
                    .map(|p| AstNode::process_node(p, script))
                    .collect::<Vec<AstNode>>();
                // Generate a null node
                AstNode {
                    kind: AstType::Null,
                    span: Some(span.clone()),
                    location: Some(location.clone()),
                    children,
                }
            }
        }
    }

    /// Processes a `script` rule to create an `AstNode`.
    /// This function processes all child pairs of the script and constructs the AST.
    /// # Arguments
    /// * `pair` - The parsing pair representing the script.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the script.
    fn process_script_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        // Extract span and location before moving `pair`
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        // Now process children
        let children = pair
            .into_inner()
            .map(|p| AstNode::process_node(p, script))
            .collect::<Vec<AstNode>>();
        // Generate the script node
        AstNode {
            kind: AstType::Script,
            span: Some(span),
            location: Some(location),
            children,
        }
    }

    /// Processes a `statement` rule to create an `AstNode`.
    /// This function matches the specific type of statement and constructs the corresponding AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the statement.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the statement.
    fn process_statement_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        let inner_pair = pair.into_inner().next().unwrap();
        let span = Self::convert_pest_span_to_span(inner_pair.as_span());
        let location = Self::convert_pest_span_to_location(inner_pair.as_span(), script);
        match inner_pair.as_rule() {
            Rule::declaration => AstNode::process_declaration_rule(inner_pair, script),
            Rule::assignment => AstNode::process_assignment_rule(inner_pair, script),
            Rule::expression => AstNode::process_expression_rule(inner_pair, script),
            Rule::include => AstNode::process_include_rule(inner_pair, script),
            Rule::import => AstNode::process_import_rule(inner_pair, script),
            _ => {
                report!(
                    Level::Warning,
                    format!("Unhandled statement rule: {:?}", inner_pair.as_rule()),
                    Some("mainstage.parser.processor.statement".to_string()),
                    Some(span.clone()),
                    Some(location.clone())
                );
                AstNode {
                    kind: AstType::Null,
                    span: Some(span.clone()),
                    location: Some(location.clone()),
                    children: vec![],
                }
            }
        }
    }

    /// Processes an `include` rule to create an `AstNode`.
    /// This function extracts the path from the include statement and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the include statement.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the include statement.
    fn process_include_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let path_pair = inner_rules.next().unwrap(); // Get the string pair
        let path = Cow::from(path_pair.as_str().trim_matches('"')); // Remove quotes
        AstNode {
            kind: AstType::Include { path },
            span: Some(span),
            location: Some(location),
            children: vec![],
        }
    }

    /// Processes an `import` rule to create an `AstNode`.
    /// This function extracts the path and alias from the import statement and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the import statement.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the import statement.
    fn process_import_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let path_pair = inner_rules.next().unwrap(); // Get the string pair
        let alias_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let path = Cow::from(path_pair.as_str().trim_matches('"')); // Remove quotes
        let alias = Cow::from(alias_pair.as_str());
        AstNode {
            kind: AstType::Import { path, alias },
            span: Some(span),
            location: Some(location),
            children: vec![],
        }
    }

    /// Processes a `declaration` rule to create an `AstNode`.
    /// This function delegates to the appropriate declaration processing function based on the inner rule.
    /// # Arguments
    /// * `pair` - The parsing pair representing the declaration.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the declaration.
    fn process_declaration_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        let inner_pair = pair.into_inner().next().unwrap();
        let span = Self::convert_pest_span_to_span(inner_pair.as_span());
        let location = Self::convert_pest_span_to_location(inner_pair.as_span(), script);
        match inner_pair.as_rule() {
            Rule::workspace_decl => AstNode::process_workspace_decl_rule(inner_pair, script),
            Rule::project_decl => AstNode::process_project_decl_rule(inner_pair, script),
            Rule::stage_decl => AstNode::process_stage_decl_rule(inner_pair, script),
            Rule::task_decl => AstNode::process_task_decl_rule(inner_pair, script),
            _ => {
                report!(
                    Level::Warning,
                    format!("Unhandled declaration rule: {:?}", inner_pair.as_rule()),
                    Some("mainstage.parser.processor.declaration".to_string()),
                    Some(span.clone()),
                    Some(location.clone())
                );
                AstNode {
                    kind: AstType::Null,
                    span: Some(span),
                    location: Some(location),
                    children: vec![],
                }
            }
        }
    }

    /// Processes an `assignment` rule to create an `AstNode`.
    /// This function extracts the left and right sides of the assignment and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the assignment.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the assignment.
    fn process_assignment_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let inner_rules = pair.into_inner();
        AstNode {
            kind: AstType::Assignment,
            span: Some(span),
            location: Some(location),
            children: vec![
                Self::process_identifier_rule(inner_rules.clone().nth(0).unwrap(), script),
                Self::process_expression_rule(inner_rules.clone().nth(1).unwrap(), script),
            ],
        }
    }

    /// Processes an `expression` rule to create an `AstNode`.
    /// This function matches the specific type of expression and constructs the corresponding AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the expression.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the expression.
    fn process_expression_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        let inner_pair = pair.into_inner().next().unwrap();
        let span = Self::convert_pest_span_to_span(inner_pair.as_span());
        let location = Self::convert_pest_span_to_location(inner_pair.as_span(), script);
        match inner_pair.as_rule() {
            Rule::identifier => Self::process_identifier_rule(inner_pair, script),
            Rule::value => Self::process_value_rule(inner_pair, script),
            _ => {
                report!(
                    Level::Warning,
                    format!("Unhandled expression rule: {:?}", inner_pair.as_rule()),
                    Some("mainstage.parser.processor.expression".to_string()),
                    Some(span.clone()),
                    Some(location.clone())
                );
                AstNode {
                    kind: AstType::Null,
                    span: Some(span),
                    location: Some(location),
                    children: vec![],
                }
            }
        }
    }

    /// Processes an `identifier` rule to create an `AstNode`.
    /// This function extracts the name from the identifier and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the identifier.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the identifier.
    fn process_identifier_rule(
        pair: pest::iterators::Pair<'a, Rule>,
        script: &Script,
    ) -> AstNode<'a> {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let name = Cow::from(pair.as_str());
        AstNode {
            kind: AstType::Identifier { name },
            span: Some(span),
            location: Some(location),
            children: vec![],
        }
    }

    /// Processes a `value` rule to create an `AstNode`.
    /// This function matches the specific type of value and constructs the corresponding AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the value.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the value.
    fn process_value_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
        let inner_pair = pair.into_inner().next().unwrap();
        let span = Self::convert_pest_span_to_span(inner_pair.as_span());
        let location = Self::convert_pest_span_to_location(inner_pair.as_span(), script);
        match inner_pair.as_rule() {
            Rule::array => AstNode {
                kind: AstType::Array,
                span: Some(span),
                location: Some(location),
                children: inner_pair
                    .into_inner()
                    .map(|p| Self::process_expression_rule(p, script))
                    .collect::<Vec<AstNode>>(),
            },
            Rule::shell_string => {
                let mut inner_rules = inner_pair.into_inner();
                let shell_pair = inner_rules.next().unwrap(); // Get the shell part
                let command_pair = inner_rules.next().unwrap(); // Get the command part
                let shell = Cow::from(shell_pair.as_str());
                let command = Cow::from(command_pair.as_str());
                AstNode {
                    kind: AstType::ShellCommand { shell, command },
                    span: Some(span),
                    location: Some(location),
                    children: vec![],
                }
            }
            Rule::string => {
                let value = Cow::from(inner_pair.as_str().trim_matches('"')); // Remove quotes
                AstNode {
                    kind: AstType::String { value },
                    span: Some(span),
                    location: Some(location),
                    children: vec![],
                }
            }
            Rule::number => {
                let value = inner_pair.as_str().parse::<f64>().unwrap_or(0.0);
                AstNode {
                    kind: AstType::Number { value },
                    span: Some(span),
                    location: Some(location),
                    children: vec![],
                }
            }
            Rule::boolean => {
                let value = inner_pair.as_str() == "true";
                AstNode {
                    kind: AstType::Boolean { value },
                    span: Some(span),
                    location: Some(location),
                    children: vec![],
                }
            }
            _ => {
                report!(
                    Level::Warning,
                    format!("Unhandled value rule: {:?}", inner_pair.as_rule()),
                    Some("mainstage.parser.processor.value".to_string()),
                    Some(span.clone()),
                    Some(location.clone())
                );
                AstNode {
                    kind: AstType::Null,
                    span: Some(span),
                    location: Some(location),
                    children: vec![],
                }
            }
        }
    }

    /// Processes a `workspace` declaration rule to create an `AstNode`.
    /// This function extracts the name from the workspace declaration and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the workspace declaration.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the workspace declaration.
    fn process_body(pairs: pest::iterators::Pairs<'a, Rule>, script: &Script) -> Vec<AstNode<'a>> {
        pairs
            .filter_map(|p| {
                Some(Self::process_statement_rule(
                    p.clone().into_inner().next().unwrap(),
                    script,
                ))
            })
            .collect()
    }

    /// Processes a `workspace` declaration rule to create an `AstNode`.
    /// This function extracts the name from the workspace declaration and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the workspace declaration.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the workspace declaration.
    fn process_workspace_decl_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let name_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let name = Cow::from(name_pair.as_str());
        AstNode {
            kind: AstType::Workspace { name },
            span: Some(span),
            location: Some(location),
            children: Self::process_body(inner_rules, script),
        }
    }

    /// Processes a `project` declaration rule to create an `AstNode`.
    /// This function extracts the name from the project declaration and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the project declaration.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the project declaration.
    fn process_project_decl_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let name_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let name = Cow::from(name_pair.as_str());
        AstNode {
            kind: AstType::Project { name },
            span: Some(span),
            location: Some(location),
            children: Self::process_body(inner_rules, script),
        }
    }

    /// Processes a `stage` declaration rule to create an `AstNode`.
    /// This function extracts the name from the stage declaration and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the stage declaration.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the stage declaration.
    fn process_stage_decl_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let name_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let name = Cow::from(name_pair.as_str());
        AstNode {
            kind: AstType::Stage { name },
            span: Some(span),
            location: Some(location),
            children: Self::process_body(inner_rules, script),
        }
    }

    /// Processes a `task` declaration rule to create an `AstNode`.
    /// This function extracts the name from the task declaration and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the task declaration.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the task declaration.
    fn process_task_decl_rule(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let name_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let name = Cow::from(name_pair.as_str());
        AstNode {
            kind: AstType::Task { name },
            span: Some(span),
            location: Some(location),
            children: Self::process_body(inner_rules, script),
        }
    }

    /// Converts the `AstNode` instance to have a different lifetime.
    /// This is useful for adapting the AST to different lifetime requirements.
    /// # Returns
    /// * An `AstNode` instance with the specified lifetime.
    pub fn into_lifetime(self) -> AstNode<'static> {
        AstNode {
            kind: self.kind.into_lifetime(),
            span: self.span,
            location: self.location.map(|loc| loc.into_owned()), // Convert location to owned
            children: self
                .children
                .into_iter()
                .map(|child| child.into_lifetime())
                .collect(),
        }
    }

    /// Converts the `AstNode` instance into an owned version.
    /// This is useful for ensuring that the AST node owns its data.
    /// # Returns
    /// * An `AstNode` instance with owned data.
    pub fn into_owned(self) -> AstNode<'static> {
        AstNode {
            kind: self.kind.into_owned(),
            span: self.span,
            location: self.location.map(|loc| loc.into_owned()),
            children: self
                .children
                .into_iter()
                .map(|child| child.into_owned())
                .collect(),
        }
    }
}
