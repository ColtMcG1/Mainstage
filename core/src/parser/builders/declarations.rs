use crate::parser::{ast::AstNode, driver::Rule};
use crate::scripts::script::Script;
use crate::parser::types::AstType;
use crate::parser::builders::expressions::{process_attributes_rule, process_arguments_rule};
use crate::parser::builders::utils::{null_node};

pub(crate) fn process_declaration_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let node = pair.into_inner().next().unwrap();
    match node.as_rule() {
        Rule::workspace_decl => process_workspace_decl_rule(node, script),
        Rule::project_decl   => process_project_decl_rule(node, script),
        Rule::stage_decl     => process_stage_decl_rule(node, script),
        Rule::task_decl      => process_task_decl_rule(node, script),
        _ => null_node(&node, script),
    }
}

pub(crate) fn process_workspace_decl_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut inner = pair.into_inner().peekable();
    let attributes = if inner.peek().map(|p| p.as_rule()) == Some(Rule::attributes) {
        process_attributes_rule(inner.next().unwrap(), script)
    } else { vec![] };
    let name_pair = inner.next().expect("workspace name");
    let name = name_pair.as_str();
    let body_pair = inner.next().expect("workspace_body");
    let children = body_pair.into_inner()
        .map(|p| AstNode::process_node(p, script))
        .collect();
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Workspace { name: name.into() },
        span: Some(span),
        location: Some(location),
        children,
        attributes,
    }
}

// New: project
pub(crate) fn process_project_decl_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut inner = pair.into_inner().peekable();
    let attributes = if inner.peek().map(|p| p.as_rule()) == Some(Rule::attributes) {
        process_attributes_rule(inner.next().unwrap(), script)
    } else { vec![] };
    let name_pair = inner.next().expect("project name");
    let name = name_pair.as_str();
    let body_pair = inner.next().expect("project_body");
    let children = body_pair.into_inner()
        .map(|p| AstNode::process_node(p, script))
        .collect();
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Project { name: name.into() },
        span: Some(span),
        location: Some(location),
        children,
        attributes,
    }
}

// New: stage
pub(crate) fn process_stage_decl_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: & Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut inner = pair.into_inner().peekable();
    let attributes = if inner.peek().map(|p| p.as_rule()) == Some(Rule::attributes) {
        process_attributes_rule(inner.next().unwrap(), script)
    } else { vec![] };
    let name_pair = inner.next().expect("stage name");
    let name = name_pair.as_str();
    // params
    let params_pair = inner.next().expect("stage params list or )");
    let params = if params_pair.as_rule() == Rule::arguments {
        process_arguments_rule(params_pair, script)
    } else { vec![] };
    // body
    let body_pair = inner.find(|p| p.as_rule() == Rule::stage_body).expect("stage body");
    let children = body_pair.into_inner()
        .map(|p| AstNode::process_node(p, script))
        .collect();
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Stage { name: name.into(), params },
        span: Some(span),
        location: Some(location),
        children,
        attributes,
    }
}

// New: task
pub(crate) fn process_task_decl_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: & Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut inner = pair.into_inner().peekable();
    let attributes = if inner.peek().map(|p| p.as_rule()) == Some(Rule::attributes) {
        process_attributes_rule(inner.next().unwrap(), script)
    } else { vec![] };
    let name_pair = inner.next().expect("task name");
    let name = name_pair.as_str();
    let params_pair = inner.next().expect("task params or )");
    let params = if params_pair.as_rule() == Rule::arguments {
        process_arguments_rule(params_pair, script)
    } else { vec![] };
    let body_pair = inner.find(|p| p.as_rule() == Rule::task_body).expect("task body");
    let children = body_pair.into_inner()
        .map(|p| AstNode::process_node(p, script))
        .collect();
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Task { name: name.into(), params },
        span: Some(span),
        location: Some(location),
        children,
        attributes,
    }
}