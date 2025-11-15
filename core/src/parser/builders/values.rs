use crate::parser::{ast::AstNode, driver::Rule, types::AstType};
use crate::scripts::script::Script;
use crate::parser::attributes::Attribute;

pub(crate) fn process_value_rule<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let inner_pair = pair.into_inner().next().unwrap();
    match inner_pair.as_rule() {
        Rule::number => number(inner_pair, script),
        Rule::string => string(inner_pair, script),
        Rule::boolean => boolean(inner_pair, script),
        Rule::array => array(inner_pair, script),
        Rule::shell_string => shell(inner_pair, script),
        _ => crate::parser::builders::utils::unhandled_rule(inner_pair, script)
    }
}

fn number<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let s = pair.as_str().trim();
    let value = s.parse::<f64>().unwrap_or(0.0);
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Number { value },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![Attribute::new("value".into(), value.to_string())],
    }
}

fn string<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    // Strip the quotes
    let text = pair.as_str();
    let val = if text.len() >= 2 { &text[1..text.len()-1] } else { text };
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Str { value: val.into() },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

fn boolean<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let value = matches!(pair.as_str(), "true");
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Bool { value },
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

fn array<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let children = pair.into_inner()
        .filter(|p| p.as_rule() == Rule::expression)
        .map(|p| crate::parser::builders::expressions::process_expression_rule(p, script))
        .collect::<Vec<_>>();
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Array,
        span: Some(span),
        location: Some(location),
        children,
        attributes: vec![],
    }
}

fn shell<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut it = pair.into_inner();
    let shell = it.next().map(|p| p.as_str().to_string()).unwrap_or_default(); // shell_prefix
    let quoted = it.next().map(|p| p.as_str().to_string()).unwrap_or_default(); // string
    let cmd = if quoted.len() >= 2 { quoted[1..quoted.len()-1].to_string() } else { quoted.clone() };
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::ShellCmd { shell: shell.clone().into(), command: cmd.clone().into() }, // surfaced as string value
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}
