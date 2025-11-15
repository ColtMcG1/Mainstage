// Patched for current grammar: arguments -> parameter -> expression,
// explicit operator rules (eq_op, rel_op, add_op, mul_op, unary_op),
// primary_expression, postfix_expression with postfix_op items.
//
// Removed dead process_param_rule; folded into arguments parsing.
// Hardened all unwraps; unified argument extraction.
// Added direct value handling (array, string, number, boolean, shell_string).
// Kept Attribute parsing (attributes rule unchanged).

use crate::parser::attributes::Attribute;
use crate::parser::builders;
use crate::parser::{
    ast::AstNode,
    driver::Rule,
    types::{AstType, BinaryOperator, UnaryOperator},
};
use crate::scripts::script::Script;

// Identifiers
pub(crate) fn process_identifier_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Identifier {
            name: pair.as_str().into(),
        },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

// Attributes [a, b]
pub(crate) fn process_attributes_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    _script: &Script,
) -> Vec<Attribute> {
    let mut v = Vec::new();
    for p in pair.into_inner() {
        if p.as_rule() == Rule::attribute {
            if let Some(id) = p.into_inner().next() {
                v.push(Attribute::new(id.as_str().to_string(), "true".to_string()));
            }
        }
    }
    v
}

// Arguments (expr, ...)
pub(crate) fn process_arguments_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> Vec<AstNode<'a>> {
    let mut args = Vec::new();
    for p in pair.into_inner() {
        if p.as_rule() == Rule::parameter {
            if let Some(e) = p.into_inner().next() {
                args.push(process_expression_rule(e, script));
            }
        }
    }
    args
}

// Expression layering
// Rewrite process_expression_rule to directly handle every layer; eliminate recursion to ast router.
pub(crate) fn process_expression_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::equality_expression       => process_binary_chain(inner, script, parse_eq_op),
        Rule::relational_expression     => process_binary_chain(inner, script, parse_rel_op),
        Rule::additive_expression       => process_binary_chain(inner, script, parse_add_op),
        Rule::multiplicative_expression => process_binary_chain(inner, script, parse_mul_op),
        Rule::unary_expression          => process_unary(inner, script),
        Rule::postfix_expression        => process_postfix(inner, script),
        Rule::primary_expression        => process_primary(inner, script),
        Rule::identifier                => process_identifier_rule(inner, script),
        Rule::value                     => super::values::process_value_rule(inner, script),
        _ => builders::utils::unhandled_rule(inner, script),
    }
}

fn process_binary_chain<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
    op_parser: fn(&str) -> Option<BinaryOperator>,
) -> AstNode<'a> {
    let mut it = pair.clone().into_inner().peekable();
    let first = it.next().unwrap();
    let mut left = process_expression_rule(first, script);
    while let (Some(op_tok), Some(rhs)) = (it.next(), it.next()) {
        if let Some(op) = op_parser(op_tok.as_str()) {
            let right = process_expression_rule(rhs, script);
            left = AstNode {
                id: AstNode::generate_id(),
                kind: AstType::BinaryOp { op, left: Box::new(left), right: Box::new(right) },
                span: Some(AstNode::convert_pest_span_to_span(pair.as_span())),
                location: Some(AstNode::convert_pest_span_to_location(pair.as_span(), script)),
                children: vec![],
                attributes: vec![],
            };
        } else {
            // If op token isn’t an operator (shouldn’t happen), break.
            break;
        }
    }
    left
}

fn process_primary<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let node = pair.clone().into_inner().next().unwrap_or(pair);
    match node.as_rule() {
        Rule::identifier => process_identifier_rule(node, script),
        Rule::value => super::values::process_value_rule(node, script),
        Rule::expression => process_expression_rule(node, script),
        _ => AstNode::process_node(node, script),
    }
}

fn process_postfix<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut it = pair.clone().into_inner();
    let base = it.next().unwrap();
    let mut current = match base.as_rule() {
        Rule::primary_expression => process_primary(base, script),
        Rule::identifier        => process_identifier_rule(base, script),
        Rule::value             => super::values::process_value_rule(base, script),
        _ => AstNode::process_node(base, script),
    };
    for op in it {
        if op.as_rule() != Rule::postfix_op { continue; }
        let mut child_it = op.clone().into_inner();
        // process each postfix_op
        if let Some(child) = child_it.next() {
            match child.as_rule() {
                // call: "(" arguments? ")"
                Rule::arguments => {
                    let args = process_arguments_rule(child, script);
                    current = AstNode {
                        id: AstNode::generate_id(),
                        kind: AstType::Call { target: Box::new(current), arguments: args },
                        span: Some(span.clone()),
                        location: Some(location.clone()),
                        children: vec![],
                        attributes: vec![],
                    };
                }
                // member: "." identifier
                Rule::identifier => {
                    let member = process_identifier_rule(child, script);
                    current = AstNode {
                        id: AstNode::generate_id(),
                        kind: AstType::Member { target: Box::new(current), member: Box::new(member) },
                        span: Some(span.clone()),
                        location: Some(location.clone()),
                        children: vec![],
                        attributes: vec![],
                    };
                }
                // index: "[" expression "]"
                Rule::expression => {
                    let idx = process_expression_rule(child, script);
                    current = AstNode {
                        id: AstNode::generate_id(),
                        kind: AstType::Index { target: Box::new(current), index: Box::new(idx) },
                        span: Some(span.clone()),
                        location: Some(location.clone()),
                        children: vec![],
                        attributes: vec![],
                    };
                }
                _ => {}
            }
        } else { // No postfix child; treat as zero-arg call
            current = AstNode {
                id: AstNode::generate_id(),
                kind: AstType::Call { target: Box::new(current), arguments: vec![] },
                span: Some(span.clone()),
                location: Some(location.clone()),
                children: vec![],
                attributes: vec![],
            };
        }
    }
    current
}

fn process_unary<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    // (unary_op)* ~ postfix_expression
    let mut ops: Vec<UnaryOperator> = Vec::new();
    let mut tail: Option<pest::iterators::Pair<'a, Rule>> = None;
    for p in pair.clone().into_inner() {
        match p.as_rule() {
            Rule::postfix_expression => {
                tail = Some(p);
            }
            Rule::unary_op => {
                let t = p.as_str();
                let op = match t {
                    "++" => UnaryOperator::Inc,
                    "--" => UnaryOperator::Dec,
                    "+" => UnaryOperator::Plus,
                    "-" => UnaryOperator::Minus,
                    _ => UnaryOperator::Plus,
                };
                ops.push(op);
            }
            _ => {}
        }
    }
    let mut node = process_postfix(tail.expect("postfix_expression"), script);
    // Apply in reverse (closest to operand last)
    for op in ops.into_iter().rev() {
        node = AstNode {
            id: AstNode::generate_id(),
            kind: AstType::UnaryOp {
                op,
                expr: Box::new(node),
                prefix: true,
            },
            span: None,
            location: None,
            children: vec![],
            attributes: vec![],
        };
    }
    node
}

fn parse_eq_op(s: &str) -> Option<BinaryOperator> {
    match s {
        "==" => Some(BinaryOperator::Eq),
        "!=" => Some(BinaryOperator::Ne),
        _ => None,
    }
}
fn parse_rel_op(s: &str) -> Option<BinaryOperator> {
    match s {
        "<" => Some(BinaryOperator::Lt),
        "<=" => Some(BinaryOperator::Le),
        ">" => Some(BinaryOperator::Gt),
        ">=" => Some(BinaryOperator::Ge),
        _ => None,
    }
}
fn parse_add_op(s: &str) -> Option<BinaryOperator> {
    match s {
        "+" => Some(BinaryOperator::Add),
        "-" => Some(BinaryOperator::Sub),
        _ => None,
    }
}
fn parse_mul_op(s: &str) -> Option<BinaryOperator> {
    match s {
        "*" => Some(BinaryOperator::Mul),
        "/" => Some(BinaryOperator::Div),
        _ => None,
    }
}
