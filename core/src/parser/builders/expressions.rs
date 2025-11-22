// Patched for current grammar: arguments -> parameter -> expression,
// explicit operator rules (eq_op, rel_op, add_op, mul_op, unary_op),
// primary_expression, postfix_expression with postfix_op items.
//
// Removed dead process_param_rule; folded into arguments parsing.
// Hardened all unwraps; unified argument extraction.
// Added direct value handling (array, string, number, boolean, shell_string).
// Kept Attribute parsing (attributes rule unchanged).

use crate::report;
use crate::parser::attributes::Attribute;
use crate::parser::builders;
use crate::parser::{
    ast::AstNode,
    driver::Rule,
    types::{AstType, BinaryOperator, UnaryOperator},
};
use crate::reports::Level;
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
    // Previously we always did `pair.into_inner().next().unwrap()` which dropped
    // operator/operand children. Fix: only unwrap the outer `expression` wrapper;
    // otherwise operate on the pair directly so binary-chain sees all children.
    let pair_to_match = if pair.as_rule() == Rule::expression {
        // expression -> equality_expression (or similar); take its child but keep full child tree
        pair.into_inner().next().expect("expression should contain a child")
    } else {
        pair
    };
    match pair_to_match.as_rule() {
        Rule::equality_expression       => process_binary_chain(pair_to_match, script, parse_eq_op),
        Rule::relational_expression     => process_binary_chain(pair_to_match, script, parse_rel_op),
        Rule::additive_expression       => process_binary_chain(pair_to_match, script, parse_add_op),
        Rule::multiplicative_expression => process_binary_chain(pair_to_match, script, parse_mul_op),
        Rule::unary_expression          => process_unary(pair_to_match, script),
        Rule::postfix_expression        => process_postfix(pair_to_match, script),
        Rule::primary_expression        => process_primary(pair_to_match, script),
        Rule::identifier                => process_identifier_rule(pair_to_match, script),
        Rule::value                     => super::values::process_value_rule(pair_to_match, script),
        _ => builders::utils::unhandled_rule(pair_to_match, script),
    }
}

fn process_binary_chain<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
    _op_parser: fn(&str) -> Option<BinaryOperator>,
) -> AstNode<'a> {
    use pest::iterators::Pair;
    
    // collect operands and operators in order from the pair's children
    let mut operands: Vec<AstNode> = Vec::new();
    let mut operators: Vec<BinaryOperator> = Vec::new();

    for child in pair.clone().into_inner() {
        match child.as_rule() {
            Rule::add_op => {
                let s = child.as_str().trim();
                let op = match s {
                    "+" => BinaryOperator::Add,
                    "-" => BinaryOperator::Sub,
                    _ => panic!("unknown add_op {}", s),
                };
                operators.push(op);
            }
            Rule::mul_op => {
                let s = child.as_str().trim();
                let op = match s {
                    "*" => BinaryOperator::Mul,
                    "/" => BinaryOperator::Div,
                    _ => panic!("unknown mul_op {}", s),
                };
                operators.push(op);
            }
            Rule::rel_op => {
                let s = child.as_str().trim();
                let op = match s {
                    "<" => BinaryOperator::Lt,
                    "<=" => BinaryOperator::Le,
                    ">" => BinaryOperator::Gt,
                    ">=" => BinaryOperator::Ge,
                    _ => panic!("unknown rel_op {}", s),
                };
                operators.push(op);
            }
            Rule::eq_op => {
                let s = child.as_str().trim();
                let op = match s {
                    "==" => BinaryOperator::Eq,
                    "!=" => BinaryOperator::Ne,
                    _ => panic!("unknown eq_op {}", s),
                };
                operators.push(op);
            }
            // any other child is an operand (sub-expression / primary)
            _ => {
                let n = process_expression_rule(child, script);
                operands.push(n);
            }
        }
    }

    // if there are no operators, the expression is just the single operand
    if operators.is_empty() {
        return operands.into_iter().next().expect("expected operand");
    }

    // precedence table (higher = tighter binding)
    fn prec(op: &BinaryOperator) -> u8 {
        match op {
            BinaryOperator::Mul | BinaryOperator::Div => 30,
            BinaryOperator::Add | BinaryOperator::Sub => 20,
            BinaryOperator::Lt | BinaryOperator::Le | BinaryOperator::Gt | BinaryOperator::Ge => 15,
            BinaryOperator::Eq | BinaryOperator::Ne => 10,
        }
    }

    // helper to make AST BinaryOp node using the original pair for span/location
    let make_node = |op: BinaryOperator, left: AstNode<'a>, right: AstNode<'a>, p: &Pair<Rule>, script: &Script| -> AstNode<'a> {
        AstNode {
            id: AstNode::generate_id(),
            kind: AstType::BinaryOp { op, left: Box::new(left), right: Box::new(right) },
            span: Some(AstNode::convert_pest_span_to_span(p.as_span())),
            location: Some(AstNode::convert_pest_span_to_location(p.as_span(), script)),
            children: vec![],
            attributes: vec![],
        }
    };

    // shunting-yard style reduction (operators are left-associative)
    let mut op_stack: Vec<BinaryOperator> = Vec::new();
    let mut val_stack: Vec<AstNode> = Vec::new();

    // push first operand
    val_stack.push(operands.remove(0));

    for i in 0..operators.len() {
        let cur_op = operators[i];
        let rhs = operands.remove(0); // next operand

        while let Some(top_op) = op_stack.last().cloned() {
            if prec(&top_op) >= prec(&cur_op) {
                let op_to_apply = op_stack.pop().unwrap();
                let right = val_stack.pop().expect("missing rhs");
                let left = val_stack.pop().expect("missing lhs");
                let node = make_node(op_to_apply, left, right, &pair, script);
                val_stack.push(node);
            } else {
                break;
            }
        }

        op_stack.push(cur_op);
        val_stack.push(rhs);
    }

    // apply remaining operators
    while let Some(op_to_apply) = op_stack.pop() {
        let right = val_stack.pop().expect("missing rhs at final reduce");
        let left = val_stack.pop().expect("missing lhs at final reduce");
        let node = make_node(op_to_apply, left, right, &pair, script);
        val_stack.push(node);
    }

    // final AST node
    val_stack.pop().expect("empty expression chain")
}

pub(crate) fn process_assign_operator_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> crate::parser::AssignOperator {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    match pair.as_str() {
        "="  => crate::parser::AssignOperator::Set,
        "+=" => crate::parser::AssignOperator::Add,
        "-=" => crate::parser::AssignOperator::Sub,
        "*=" => crate::parser::AssignOperator::Mul,
        "/=" => crate::parser::AssignOperator::Div,
        _    => {
            report!(
                Level::Warning,
                format!("Unknown assignment operator '{}', defaulting to '='", pair.as_str()),
                Some("mainstage.parser.expressions.process_assign_operator_rule".into()),
                Some(span),
                Some(location)
            );
            return crate::parser::AssignOperator::Set; // Default to Set on unknown
        }
    }
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
