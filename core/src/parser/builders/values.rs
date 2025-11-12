use crate::parser::{ast::AstNode, driver::Rule};
use crate::scripts::script::Script;
use crate::parser::types::AstType;
use crate::parser::attributes::Attribute;
use std::borrow::Cow;

pub(crate) fn process_value_rule<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let inner = pair.into_inner().next().unwrap();
    let span = AstNode::convert_pest_span_to_span(inner.as_span());
    let location = AstNode::convert_pest_span_to_location(inner.as_span(), script);
    match inner.as_rule() {
        Rule::array => AstNode {
            id: AstNode::generate_id(),
            kind: AstType::Array,
            span: Some(span),
            location: Some(location),
            children: inner.into_inner().map(|p| super::expressions::process_expression_rule(p, script)).collect(),
            attributes: vec![],
        },
        Rule::shell_string => {
            let mut it = inner.into_inner();
            let shell = Cow::from(it.next().unwrap().as_str());
            let command = Cow::from(it.next().unwrap().as_str());
            AstNode {
                id: AstNode::generate_id(),
                kind: AstType::ShellCommand { shell: shell.clone(), command: command.clone() },
                span: Some(span), location: Some(location),
                children: vec![],
                attributes: vec![
                    Attribute::new("shell".to_string(), shell.to_string()),
                    Attribute::new("command".to_string(), command.to_string()),
                ],
            }
        }
        Rule::string => {
            let value = Cow::from(inner.as_str().trim_matches('"'));
            AstNode { id: AstNode::generate_id(), kind: AstType::String { value: value.clone() }, span: Some(span), location: Some(location), children: vec![], attributes: vec![Attribute::new("value".to_string(), value.to_string())] }
        }
        Rule::number => {
            let value = inner.as_str().parse::<f64>().unwrap_or(0.0);
            AstNode { id: AstNode::generate_id(), kind: AstType::Number { value }, span: Some(span), location: Some(location), children: vec![], attributes: vec![Attribute::new("value".to_string(), value.to_string())] }
        }
        Rule::boolean => {
            let value = inner.as_str() == "true";
            AstNode { id: AstNode::generate_id(), kind: AstType::Boolean { value }, span: Some(span), location: Some(location), children: vec![], attributes: vec![Attribute::new("value".to_string(), value.to_string())] }
        }
        _ => AstNode { id: AstNode::generate_id(), kind: AstType::Null, span: Some(span), location: Some(location), children: vec![], attributes: vec![] }
    }
}