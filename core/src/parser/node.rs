use std::borrow::Cow;

pub use crate::parser::types::AstType;
use crate::report;
use crate::reports::*;
use crate::scripts::script::Script;

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
pub struct AstNode {
    pub kind: AstType,
    pub span: Option<locations::Span>,
    pub location: Option<locations::Location<'static>>,
    pub children: Vec<AstNode>,
}

impl AstNode {
    /// Creates a new `AstNode` from the given parsing pairs.
    /// This function processes the pairs and constructs the AST recursively.
    /// # Arguments
    /// * `pairs` - The parsing pairs obtained from the parser.
    /// # Returns
    /// * `Ok(AstNode)` if the AST is successfully created.
    /// * `Err(Report)` if there is an error during AST creation.
    pub fn new(pairs: pest::iterators::Pairs<Rule>, script: &Script) -> Result<Self, ()> {
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
    fn process_node(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
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
                    "Unhandled rule".to_string(),
                    Some("Parser".to_string()),
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
    fn process_script_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
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
    fn process_statement_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let inner_pair = pair.into_inner().next().unwrap();
        match inner_pair.as_rule() {
            Rule::declaration => AstNode::process_declaration_rule(inner_pair, script),
            Rule::assignment => AstNode::process_assignment_rule(inner_pair, script),
            Rule::expression_statement => {
                AstNode::process_expression_statement_rule(inner_pair, script)
            }
            Rule::include => AstNode::process_include_rule(inner_pair, script),
            Rule::import => AstNode::process_import_rule(inner_pair, script),
            _ => AstNode {
                kind: AstType::Null,
                span: Some(span.clone()),
                location: Some(location.clone()),
                children: vec![],
            },
        }
    }

    /// Processes an `include` rule to create an `AstNode`.
    /// This function extracts the path from the include statement and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the include statement.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the include statement.
    fn process_include_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let path_pair = inner_rules.next().unwrap(); // Get the string pair
        let path = path_pair.as_str().trim_matches('"').to_string(); // Remove quotes
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
    fn process_import_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let path_pair = inner_rules.next().unwrap(); // Get the string pair
        let alias_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let path = path_pair.as_str().trim_matches('"').to_string(); // Remove quotes
        let alias = alias_pair.as_str().to_string();
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
    fn process_declaration_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let inner_pair = pair.into_inner().next().unwrap();
        match inner_pair.as_rule() {
            Rule::workspace_decl => AstNode::process_workspace_decl_rule(inner_pair, script),
            Rule::project_decl => AstNode::process_project_decl_rule(inner_pair, script),
            Rule::stage_decl => AstNode::process_stage_decl_rule(inner_pair, script),
            Rule::task_decl => AstNode::process_task_decl_rule(inner_pair, script),
            _ => AstNode {
                kind: AstType::Null,
                span: Some(span),
                location: Some(location),
                children: vec![],
            },
        }
    }

    /// Processes an `assignment` rule to create an `AstNode`.
    /// This function extracts the left and right sides of the assignment and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the assignment.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the assignment.
    fn process_assignment_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let left_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let right_pair = inner_rules.next().unwrap(); // Get the value or identifier pair
        let left = left_pair.as_str().to_string();
        let right = right_pair.as_str().to_string();
        AstNode {
            kind: AstType::Assignment { left, right },
            span: Some(span),
            location: Some(location),
            children: vec![],
        }
    }

    /// Processes an `expression_statement` rule to create an `AstNode`.
    /// This function delegates to the appropriate expression processing function based on the inner rule.
    /// # Arguments
    /// * `pair` - The parsing pair representing the expression statement.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the expression statement.
    fn process_expression_statement_rule(
        pair: pest::iterators::Pair<Rule>,
        script: &Script,
    ) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let inner_pair = pair.into_inner().next().unwrap();
        match inner_pair.as_rule() {
            Rule::expression => AstNode::process_expression_rule(inner_pair, script),
            _ => AstNode {
                kind: AstType::Null,
                span: Some(span),
                location: Some(location),
                children: vec![],
            },
        }
    }

    /// Processes an `expression` rule to create an `AstNode`.
    /// This function matches the specific type of expression and constructs the corresponding AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the expression.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the expression.
    fn process_expression_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let inner_pair = pair.into_inner().next().unwrap();
        match inner_pair.as_rule() {
            Rule::identifier => {
                let name = inner_pair.as_str().to_string();
                AstNode {
                    kind: AstType::Identifier { name },
                    span: Some(span),
                    location: Some(location),
                    children: vec![],
                }
            }
            Rule::shell_string => {
                let mut inner_rules = inner_pair.into_inner();
                let shell_pair = inner_rules.next().unwrap(); // Get the shell part
                let command_pair = inner_rules.next().unwrap(); // Get the command part
                let shell = shell_pair.as_str().to_string();
                let command = command_pair.as_str().to_string();
                AstNode {
                    kind: AstType::ShellCommand { shell, command },
                    span: Some(span),
                    location: Some(location),
                    children: vec![],
                }
            }
            Rule::string => {
                let value = inner_pair.as_str().trim_matches('"').to_string(); // Remove quotes
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
            _ => AstNode {
                kind: AstType::Null,
                span: Some(span),
                location: Some(location),
                children: vec![],
            },
        }
    }

    /// Processes a `workspace` declaration rule to create an `AstNode`.
    /// This function extracts the name from the workspace declaration and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the workspace declaration.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the workspace declaration.
    fn process_workspace_decl_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let name_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let name = name_pair.as_str().to_string();
        AstNode {
            kind: AstType::Workspace { name },
            span: Some(span),
            location: Some(location),
            children: vec![],
        }
    }

    /// Processes a `project` declaration rule to create an `AstNode`.
    /// This function extracts the name from the project declaration and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the project declaration.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the project declaration.
    fn process_project_decl_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let name_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let name = name_pair.as_str().to_string();
        AstNode {
            kind: AstType::Project { name },
            span: Some(span),
            location: Some(location),
            children: vec![],
        }
    }

    /// Processes a `stage` declaration rule to create an `AstNode`.
    /// This function extracts the name from the stage declaration and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the stage declaration.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the stage declaration.
    fn process_stage_decl_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let name_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let name = name_pair.as_str().to_string();
        AstNode {
            kind: AstType::Stage { name },
            span: Some(span),
            location: Some(location),
            children: vec![],
        }
    }

    /// Processes a `task` declaration rule to create an `AstNode`.
    /// This function extracts the name from the task declaration and constructs the AST node.
    /// # Arguments
    /// * `pair` - The parsing pair representing the task declaration.
    /// * `script` - The script being processed.
    /// # Returns
    /// * An `AstNode` representing the task declaration.
    fn process_task_decl_rule(pair: pest::iterators::Pair<Rule>, script: &Script) -> Self {
        let span = Self::convert_pest_span_to_span(pair.as_span());
        let location = Self::convert_pest_span_to_location(pair.as_span(), script);
        let mut inner_rules = pair.into_inner();
        let name_pair = inner_rules.next().unwrap(); // Get the identifier pair
        let name = name_pair.as_str().to_string();
        AstNode {
            kind: AstType::Task { name },
            span: Some(span),
            location: Some(location),
            children: vec![],
        }
    }
}
