use crate::parser::builders;
use crate::parser::{ast::AstNode, driver::Rule, types::AstType};
use crate::scripts::script::Script;

fn process_block<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let children = pair
        .into_inner()
        .map(|p| AstNode::process_node(p, script))
        .collect();
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Block,
        span: Some(span),
        location: Some(location),
        children,
        attributes: vec![],
    }
}


/// Converts a statement rule to a corresponding AST node.
/// # Reference
/// - [Rule::statement](crate::parser::driver::Rule::statement)
/// # Arguments
/// - `pair`: The pest pair representing the statement rule.
/// - `script`: The script context.
/// # Returns
/// An `AstNode` representing the processed statement.
pub(crate) fn process_statement_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let inner = pair.clone().into_inner().next().unwrap_or(pair);
    match inner.as_rule() {
        Rule::terminated_statement => process_terminated_stmt(inner, script),
        Rule::block => process_block(inner, script),
        Rule::loop_stmt => process_loop_stmt(inner, script),
        Rule::conditional_stmt => process_conditional_stmt(inner, script),
        _ => builders::utils::unhandled_rule(inner, script),
    }
}

/// Processes a terminated statement rule into an AST node.
/// # Reference
/// - [Rule::terminated_statement](crate::parser::driver::Rule::terminated_statement)
/// # Arguments
/// - `pair`: The pest pair representing the terminated statement rule.
/// - `script`: The script context.
/// # Returns
/// An `AstNode` representing the processed terminated statement.
fn process_terminated_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let stmt = pair.into_inner().next().unwrap();
    match stmt.as_rule() {
        Rule::return_stmt => process_return_stmt(stmt, script),
        Rule::assignment_stmt => process_assignment_stmt(stmt, script),
        Rule::include_stmt => process_include_stmt(stmt, script),
        Rule::import_stmt => process_import_stmt(stmt, script),
        Rule::expression_stmt => {
            let expr = stmt.into_inner().next().unwrap();
            builders::expressions::process_expression_rule(expr, script)
        }
        _ => builders::utils::unhandled_rule(stmt, script),
    }
}

/// Processes a return statement rule into an AST node.
/// # Reference
/// - [Rule::return_stmt](crate::parser::driver::Rule::return_stmt)
/// # Arguments
/// - `pair`: The pest pair representing the return statement rule.
/// - `script`: The script context.
/// # Returns
/// An `AstNode` representing the processed return statement.
pub(crate) fn process_return_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let expr = pair
        .into_inner()
        .find(|p| p.as_rule() == Rule::expression)
        .map(|p| builders::expressions::process_expression_rule(p, script))
        .expect("Return must return a valid expression");
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Return,
        span: Some(span),
        location: Some(location),
        children: vec![expr],
        attributes: vec![],
    }
}


/// Processes an assignment statement into an AST node.
/// # Reference
/// - [Rule::assignment_stmt](crate::parser::driver::Rule::assignment_stmt)
/// # Arguments
/// - `pair`: The pest pair representing the assignment statement rule.
/// - `script`: The script context.
/// # Returns
/// An `AstNode` representing the processed assignment statement.
pub(crate) fn process_assignment_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut lhs = None;
    let mut op = None;
    let mut rhs = None;
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::identifier => lhs = Some(builders::expressions::process_identifier_rule(p, script)),
            Rule::assign_op => op = Some(builders::expressions::process_assign_operator_rule(p, script)),
            Rule::expression => rhs = Some(builders::expressions::process_expression_rule(p, script)),
            _ => {}
        }
    }
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Assignment { op: op.expect("Assignment must have a valid assignment operator") },
        span: Some(span),
        location: Some(location),
        children: vec![
            lhs.expect("Assignment must have a valid identifier on the left hand"),
            rhs.expect("Assignment must have a valid expression on the right hand"),
        ],
        attributes: vec![],
    }
}

/// Processes an include statement rule into an AST node.
/// # Reference
/// - [Rule::include_stmt](crate::parser::driver::Rule::include_stmt)
/// # Arguments
/// - `pair`: The pest pair representing the include statement rule.
/// - `script`: The script context.
/// # Returns
/// An `AstNode` representing the processed include statement.
pub(crate) fn process_include_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let path = pair.clone().into_inner()
        .find(|p| p.as_rule() == Rule::string)
        .map(|p| builders::utils::unquote(p.as_str()).to_string())
        .unwrap_or_default();
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Include { path: path.into() },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

/// Processes an import statement rule into an AST node.
/// # Reference
/// - [Rule::import_stmt](crate::parser::driver::Rule::import_stmt)
/// # Arguments
/// - `pair`: The pest pair representing the import statement rule.
/// - `script`: The script context.
/// # Returns
/// An `AstNode` representing the processed import statement.
pub(crate) fn process_import_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut path = String::new();
    let mut alias = String::new();
    
    // import_stmt = { "import" ~ string ~ ( "as" ~ identifier )? ~ ";" }
    let mut inner_rules = pair.into_inner();

    if let Some(path_pair) = inner_rules.next() {
        if path_pair.as_rule() == Rule::string {
            path = builders::utils::unquote(path_pair.as_str()).to_string();
        }
    }

    if let Some(alias_pair) = inner_rules.next() {
        if alias_pair.as_rule() == Rule::identifier {
            alias = alias_pair.as_str().to_string();
        }
    }

    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Import { path: path.into(), alias: alias.into() },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

fn process_conditional_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::if_stmt => process_if_stmt(inner, script),
        Rule::ifelse_stmt => process_ifelse_stmt(inner, script),
        Rule::tenary_stmt => process_tenary_stmt(inner, script),
        _ => builders::utils::unhandled_rule(inner, script),
    }
}

fn process_if_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut condition = None;
    let mut body = None;
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::expression => {
                condition = Some(Box::new(builders::expressions::process_expression_rule(p, script)));
            }
            Rule::block => {
                body = Some(process_block(p, script));
            }
            _ => {}
        }
    }
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::If {
            cond: condition.expect("If statement must have a condition"),
            body: Box::new(
                body.expect("If statement must have body"),
            ),
        },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

fn process_ifelse_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut condition = None;
    let mut if_body = None;
    let mut else_body = None;
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::expression => {
                condition = Some(Box::new(builders::expressions::process_expression_rule(p, script)));
            }
            Rule::block => {
                if if_body.is_none() {
                    if_body = Some(process_block(p, script));
                } else {
                    else_body = Some(process_block(p, script));
                }
            }
            _ => {}
        }
    }
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::IfElse {
            cond: condition.expect("If-Else statement must have a condition"),
            if_body: Box::new(
                if_body.expect("If-Else statement must have if body"),
            ),
            else_body: Box::new(
                else_body.expect("If-Else statement must have else body"),
            ),
        },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

fn process_tenary_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut condition = None;
    let mut true_expr = None;
    let mut false_expr = None;
    let mut inner_rules = pair.into_inner();

    if let Some(cond_pair) = inner_rules.next() {
        if cond_pair.as_rule() == Rule::expression {
            condition = Some(Box::new(builders::expressions::process_expression_rule(cond_pair, script)));
        }
    }

    if let Some(true_pair) = inner_rules.next() {
        if true_pair.as_rule() == Rule::expression {
            true_expr = Some(builders::expressions::process_expression_rule(true_pair, script));
        }
    }

    if let Some(false_pair) = inner_rules.next() {
        if false_pair.as_rule() == Rule::expression {
            false_expr = Some(builders::expressions::process_expression_rule(false_pair, script));
        }
    }

    // Ternary is converted to IfElse AST node
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::IfElse {
            cond: condition.expect("Ternary statement must have a condition"),
            if_body: Box::new(
                true_expr.expect("Ternary statement must have true expression"),
            ),
            else_body: Box::new(
                false_expr.expect("Ternary statement must have false expression"),
            ),
        },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

/// Processes a loop statement rule into an AST node.
/// # Reference
/// - [Rule::loop_stmt](crate::parser::driver::Rule::loop_stmt)
/// # Arguments
/// - `pair`: The pest pair representing the loop statement rule.
/// - `script`: The script context.
/// # Returns
/// An `AstNode` representing the processed loop statement.
fn process_loop_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::forin_stmt => process_forin_stmt(inner, script),
        Rule::forto_stmt => process_forto_stmt(inner, script),
        Rule::while_stmt => process_while_stmt(inner, script),
        _ => builders::utils::unhandled_rule(inner, script),
    }
}

fn process_forin_stmt<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let mut iden = String::new();
    let mut iter = None;
    let mut body = None;
    
    // for "(" IDENT "in" expression ")" block
    let mut inner_rules = pair.into_inner();

    if let Some(ident_pair) = inner_rules.next() {
        if ident_pair.as_rule() == Rule::identifier {
            iden = ident_pair.as_str().to_string();
        }
    }

    if let Some(iterable_pair) = inner_rules.next() {
        if iterable_pair.as_rule() == Rule::expression {
            iter = Some(Box::new(builders::expressions::process_expression_rule(iterable_pair, script)));
        }
    }

    if let Some(block_pair) = inner_rules.next() {
        if block_pair.as_rule() == Rule::block {
            body = Some(Box::new(process_block(block_pair, script)));
        }
    }

    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Forin {
            iden: iden.into(),
            iter: iter.expect("For-in loop must have iterable expression"),
            body: body.expect("For loop must have body")
        },
        span: None,
        location: None,
        children: vec![],
        attributes: vec![],
    }
}

fn process_forto_stmt<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    // for assignment to expression block
    let mut init = None;
    let mut limt = None;
    let mut body = None;
    
    let mut inner_rules = pair.into_inner();

    // Check for init
    if let Some(first) = inner_rules.peek() {
        if first.as_rule() == Rule::assignment_expr {
            let assignment_pair = inner_rules.next().unwrap();
            init = Some(Box::new(process_assignment_stmt(assignment_pair, script)));
        }
    }

    // Check for expression (limit)
    if let Some(next) = inner_rules.peek() {
        if next.as_rule() == Rule::expression {
            let expr_pair = inner_rules.next().unwrap();
            limt = Some(Box::new(builders::expressions::process_expression_rule(expr_pair, script)));
        }
    }

    // The last part should be the block
    if let Some(block_pair) = inner_rules.next() {
        if block_pair.as_rule() == Rule::block {
            body = Some(Box::new(process_block(block_pair, script)));
        }
    }

    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Forto {
            init: init.expect("For loop must have initialization assignment"),
            limt: limt.expect("For loop must have limit expression"),
            body: body.expect("For loop must have body"),
        },
        span: None,
        location: None,
        children: vec![],
        attributes: vec![],
    }
}

fn process_while_stmt<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let mut condition = None;
    let mut body = None;
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::expression => {
                condition = Some(Box::new(builders::expressions::process_expression_rule(
                    p, script,
                )));
            }
            Rule::block => {
                body = Some(process_block(p, script));
            }
            _ => {}
        }
    }
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::While {
            cond: condition.expect("While loop must have a condition"),
            body: Box::new(
                body.expect("While loop must have body"),
            ),
        },
        span: None,
        location: None,
        children: vec![],
        attributes: vec![],
    }
}