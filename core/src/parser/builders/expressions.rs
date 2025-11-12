// Patched for current grammar: arguments -> parameter -> expression,
// explicit operator rules (eq_op, rel_op, add_op, mul_op, unary_op),
// primary_expression, postfix_expression with postfix_op items.
//
// Removed dead process_param_rule; folded into arguments parsing.
// Hardened all unwraps; unified argument extraction.
// Added direct value handling (array, string, number, boolean, shell_string).
// Kept Attribute parsing (attributes rule unchanged).

use crate::parser::attributes::Attribute;
use crate::parser::builders::utils::null_node;
use crate::parser::types::{AstType, BinaryOperator, UnaryOperator};
use crate::parser::{ast::AstNode, driver::Rule};
use crate::scripts::script::Script;
use std::borrow::Cow;

// ---- Public entry ---------------------------------------------------------
pub(crate) fn process_expression_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(first) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    match first.as_rule() {
        Rule::equality_expression       => process_equality_expression_rule(first, script),
        Rule::relational_expression     => process_relational_expression_rule(first, script),
        Rule::additive_expression       => process_additive_expression_rule(first, script),
        Rule::multiplicative_expression => process_multiplicative_expression_rule(first, script),
        Rule::unary_expression          => process_unary_expression_rule(first, script),
        Rule::postfix_expression        => process_postfix_expression_rule(first, script),
        Rule::primary_expression        => process_primary_expression_rule(first, script),
        Rule::call_expression           => process_call_expression_rule(first, script),
        Rule::identifier                => process_identifier_rule(first, script),
        Rule::value                     => super::values::process_value_rule(first, script),
        _ => null_node(&first, script),
    }
}

// ---- Statements / directives ----------------------------------------------
pub(crate) fn process_return_statement_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(expr_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Return,
        span: Some(span),
        location: Some(location),
        children: vec![process_expression_rule(expr_pair, script)],
        attributes: vec![],
    }
}

pub(crate) fn process_assignment_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(lhs_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let lhs = process_identifier_rule(lhs_pair, script);
    let Some(rhs_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let rhs = process_expression_rule(rhs_pair, script);
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Assignment,
        span: Some(span),
        location: Some(location),
        children: vec![lhs, rhs],
        attributes: vec![],
    }
}

pub(crate) fn process_include_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(path_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let path = Cow::from(path_pair.as_str().trim_matches('"'));
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Include { path: path.clone() },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![Attribute::new("path".to_string(), path.to_string())],
    }
}

pub(crate) fn process_import_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(path_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let Some(alias_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let path = Cow::from(path_pair.as_str().trim_matches('"'));
    let alias = Cow::from(alias_pair.as_str());
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Import { path: path.clone(), alias: alias.clone() },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![
            Attribute::new("path".into(), path.to_string()),
            Attribute::new("alias".into(), alias.to_string()),
        ],
    }
}

// ---- Simple atoms ---------------------------------------------------------
pub(crate) fn process_identifier_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let name = Cow::from(pair.as_str());
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Identifier { name },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

// ---- Calls / arguments ----------------------------------------------------
pub(crate) fn process_call_expression_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let pair_clone = pair.clone();
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut inner = pair.into_inner();
    let Some(id_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let target = Box::new(process_identifier_rule(id_pair, script));
    let arguments = match inner.next() {
        Some(args_pair) if args_pair.as_rule() == Rule::arguments => {
            process_arguments_rule(args_pair, script)
        }
        _ => vec![],
    };
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::CallExpression { target, arguments },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

pub(crate) fn process_arguments_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> Vec<AstNode<'a>> {
    // grammar: arguments = { parameter ~ ("," ~ parameter)* }
    pair.into_inner()
        .filter_map(|p| {
            if p.as_rule() == Rule::parameter {
                let mut inner = p.into_inner();
                inner.next().map(|expr| process_expression_rule(expr, script))
            } else {
                None
            }
        })
        .collect()
}

// ---- Attributes -----------------------------------------------------------
pub(crate) fn process_attributes_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    _script: &Script,
) -> Vec<Attribute> {
    pair.into_inner()
        .map(|p| {
            // p is attribute
            let parts = p.into_inner().collect::<Vec<_>>();
            if parts.len() == 1 {
                Attribute::new(parts[0].as_str().to_string(), "".to_string())
            } else if parts.len() == 2 {
                Attribute::new(parts[0].as_str().to_string(),
                               parts[1].as_str().trim_matches('"').to_string())
            } else {
                Attribute::new("".into(), "".into())
            }
        })
        .collect()
}

// ---- Expression layers ----------------------------------------------------
pub(crate) fn process_primary_expression_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(first) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    match first.as_rule() {
        Rule::call_expression   => process_call_expression_rule(first, script),
        Rule::identifier        => process_identifier_rule(first, script),
        Rule::value             => super::values::process_value_rule(first, script),
        Rule::expression        => process_expression_rule(first, script),
        _ => AstNode::process_node(first, script),
    }
}

pub(crate) fn process_postfix_expression_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(first) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    // Base (primary_expression)
    let mut current = match first.as_rule() {
        Rule::primary_expression => process_primary_expression_rule(first, script),
        Rule::call_expression    => process_call_expression_rule(first, script),
        Rule::identifier         => process_identifier_rule(first, script),
        Rule::value              => super::values::process_value_rule(first, script),
        _ => AstNode::process_node(first, script),
    };

    // Each remaining child is a postfix_op
    for op_pair in inner {
        if op_pair.as_rule() != Rule::postfix_op {
            continue;
        }

        let text = op_pair.as_str();
        let mut op_inner = op_pair.into_inner();

        // 1. Inc/Dec
        if text == "++" || text == "--" {
            let op = if text == "++" { UnaryOperator::Inc } else { UnaryOperator::Dec };
            current = AstNode {
                id: AstNode::generate_id(),
                kind: AstType::UnaryOp { op, expr: Box::new(current), prefix: false },
                span: Some(span.clone()),
                location: Some(location.clone()),
                children: vec![],
                attributes: vec![],
            };
            continue;
        }

        // 2. Member access: "." identifier
        if text.starts_with('.') {
            // Grammar: "." ~ identifier => one child: identifier
            if let Some(id_pair) = op_inner.next() {
                let member_ident = process_identifier_rule(id_pair, script);
                current = AstNode {
                    id: AstNode::generate_id(),
                    kind: AstType::MemberAccess {
                        target: Box::new(current),
                        member: Box::new(member_ident),
                    },
                    span: Some(span.clone()),
                    location: Some(location.clone()),
                    children: vec![],
                    attributes: vec![],
                };
            }
            continue;
        }

        // 3. Index access: "[" expression "]"
        if text.starts_with('[') {
            // Child: expression
            if let Some(expr_pair) = op_inner.next() {
                let index_expr = process_expression_rule(expr_pair, script);
                current = AstNode {
                    id: AstNode::generate_id(),
                    kind: AstType::Index {
                        target: Box::new(current),
                        index: Box::new(index_expr),
                    },
                    span: Some(span.clone()),
                    location: Some(location.clone()),
                    children: vec![],
                    attributes: vec![],
                };
            }
            continue;
        }

        // Fallback (should not happen)
    }

    current
}

fn build_left_assoc<'a>(
    first: AstNode<'a>,
    mut rest: Vec<(BinaryOperator, AstNode<'a>)>,
) -> AstNode<'a> {
    let mut acc = first;
    for (op, rhs) in rest.drain(..) {
        // Merge span if available
        let span = match (acc.span.clone(), rhs.span.clone()) {
            (Some(l), Some(r)) => Some(crate::reports::locations::Span::new(l.start, r.end)),
            (s @ Some(_), None) | (None, s @ Some(_)) => s,
            _ => None,
        };
        let location = acc.location.clone(); // keep left location
        acc = AstNode {
            id: AstNode::generate_id(),
            kind: AstType::BinaryOp { op, left: Box::new(acc), right: Box::new(rhs) },
            span,
            location,
            children: vec![],
            attributes: vec![],
        };
    }
    acc
}

pub(crate) fn process_unary_expression_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner().peekable();
    let mut ops = Vec::<UnaryOperator>::new();
    while let Some(p) = inner.peek() {
        if p.as_rule() == Rule::unary_op {
            let op = match p.as_str() {
                "+" => UnaryOperator::Plus,
                "-" => UnaryOperator::Minus,
                "++" => UnaryOperator::Inc,
                "--" => UnaryOperator::Dec,
                _ => break,
            };
            ops.push(op);
            inner.next();
        } else {
            break;
        }
    }
    let Some(base_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let base = process_postfix_expression_rule(base_pair, script);
    ops.into_iter().rev().fold(base, |acc, op| AstNode {
        id: AstNode::generate_id(),
        kind: AstType::UnaryOp {
            op,
            expr: Box::new(acc),
            prefix: true,
        },
        span: None,
        location: None,
        children: vec![],
        attributes: vec![],
    })
}

pub(crate) fn process_multiplicative_expression_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(first_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let first = process_unary_expression_rule(first_pair, script);
    let mut rest = Vec::new();
    while let Some(op_pair) = inner.next() {
        if op_pair.as_rule() != Rule::mul_op {
            break;
        }
        let Some(rhs_pair) = inner.next() else { break; };
        let op = match op_pair.as_str() {
            "*" => BinaryOperator::Mul,
            "/" => BinaryOperator::Div,
            _ => break,
        };
        let rhs = process_unary_expression_rule(rhs_pair, script);
        rest.push((op, rhs));
    }
    build_left_assoc(first, rest)
}

pub(crate) fn process_additive_expression_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(first_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let first = process_multiplicative_expression_rule(first_pair, script);
    let mut rest = Vec::new();
    while let Some(op_pair) = inner.next() {
        if op_pair.as_rule() != Rule::add_op {
            break;
        }
        let Some(rhs_pair) = inner.next() else { break; };
        let op = match op_pair.as_str() {
            "+" => BinaryOperator::Add,
            "-" => BinaryOperator::Sub,
            _ => break,
        };
        let rhs = process_multiplicative_expression_rule(rhs_pair, script);
        rest.push((op, rhs));
    }
    build_left_assoc(first, rest)
}

pub(crate) fn process_relational_expression_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(first_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let first = process_additive_expression_rule(first_pair, script);
    let mut rest = Vec::new();
    while let Some(op_pair) = inner.next() {
        if op_pair.as_rule() != Rule::rel_op {
            break;
        }
        let Some(rhs_pair) = inner.next() else { break; };
        let op = match op_pair.as_str() {
            "<" => BinaryOperator::Lt,
            ">" => BinaryOperator::Gt,
            "<=" => BinaryOperator::Le,
            ">=" => BinaryOperator::Ge,
            _ => break,
        };
        let rhs = process_additive_expression_rule(rhs_pair, script);
        rest.push((op, rhs));
    }
    build_left_assoc(first, rest)
}

pub(crate) fn process_equality_expression_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(first_pair) = inner.next() else {
        return null_node(&pair_clone, script);
    };
    let first = process_relational_expression_rule(first_pair, script);
    let mut rest = Vec::new();
    while let Some(op_pair) = inner.next() {
        if op_pair.as_rule() != Rule::eq_op {
            break;
        }
        let Some(rhs_pair) = inner.next() else { break; };
        let op = match op_pair.as_str() {
            "==" => BinaryOperator::Eq,
            "!=" => BinaryOperator::Neq,
            _ => break,
        };
        let rhs = process_relational_expression_rule(rhs_pair, script);
        rest.push((op, rhs));
    }
    build_left_assoc(first, rest)
}
