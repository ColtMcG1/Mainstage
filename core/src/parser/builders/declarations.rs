// Ensure decl bodies read a generic block { ... }.
use crate::parser::{ast::AstNode, driver::Rule, types::AstType};
use crate::parser::builders::expressions::{process_arguments_rule, process_attributes_rule};
use crate::scripts::script::Script;

fn collect_block<'a>(p: pest::iterators::Pair<'a, Rule>, script: &Script) -> Vec<AstNode<'a>> {
    match p.as_rule() {
        Rule::block => p.into_inner().map(|c| AstNode::process_node(c, script)).collect(),
        _ => vec![],
    }
}

pub(crate) fn process_declaration_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    // declaration wraps the specific decl
    let node = pair.into_inner().next().unwrap();
    match node.as_rule() {
        Rule::workspace_decl => process_workspace(node, script),
        Rule::project_decl   => process_project(node, script),
        Rule::stage_decl     => process_stage(node, script),
        Rule::task_decl      => process_task(node, script),
        _ => crate::parser::builders::utils::unhandled_rule(node, script),
    }
}

fn process_workspace<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut it = pair.into_inner().peekable();
    let attributes = if it.peek().map(|p| p.as_rule()) == Some(Rule::attributes) {
        process_attributes_rule(it.next().unwrap(), script)
    } else { vec![] };
    let name = it.next().expect("workspace name").as_str();
    let body = it.next().expect("workspace block");
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Workspace { name: name.into() },
        span: Some(span),
        location: Some(location),
        children: collect_block(body, script),
        attributes,
    }
}

fn process_project<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut it = pair.into_inner().peekable();
    let attributes = if it.peek().map(|p| p.as_rule()) == Some(Rule::attributes) {
        process_attributes_rule(it.next().unwrap(), script)
    } else { vec![] };
    let name = it.next().expect("project name").as_str();
    let body = it.next().expect("project block");
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Project { name: name.into() },
        span: Some(span),
        location: Some(location),
        children: collect_block(body, script),
        attributes,
    }
}

fn process_stage<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut it = pair.into_inner().peekable();
    let attributes = if it.peek().map(|p| p.as_rule()) == Some(Rule::attributes) {
        process_attributes_rule(it.next().unwrap(), script)
    } else { vec![] };
    let name = it.next().expect("stage name").as_str();
    let mut params = Vec::new();
    if it.peek().map(|p| p.as_rule()) == Some(Rule::arguments) {
        params = process_arguments_rule(it.next().unwrap(), script);
    }
    let body = it.next().expect("stage block");
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Stage { name: name.into(), params },
        span: Some(span),
        location: Some(location),
        children: collect_block(body, script),
        attributes,
    }
}

fn process_task<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut it = pair.into_inner().peekable();
    let attributes = if it.peek().map(|p| p.as_rule()) == Some(Rule::attributes) {
        process_attributes_rule(it.next().unwrap(), script)
    } else { vec![] };
    let name = it.next().expect("task name").as_str();
    let mut params = Vec::new();
    if it.peek().map(|p| p.as_rule()) == Some(Rule::arguments) {
        params = process_arguments_rule(it.next().unwrap(), script);
    }
    let body = it.next().expect("task block");
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Task { name: name.into(), params },
        span: Some(span),
        location: Some(location),
        children: collect_block(body, script),
        attributes,
    }
}