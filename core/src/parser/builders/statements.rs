use crate::parser::{ast::AstNode, driver::Rule};
use crate::scripts::script::Script;
use crate::parser::builders;

pub(crate) fn process_statement_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let pair_clone = pair.clone();
    let mut inner = pair.into_inner();
    let Some(node) = inner.next() else {
        return builders::utils::null_node(&pair_clone, script);
    };
    match node.as_rule() {
        Rule::return_statement => builders::expressions::process_return_statement_rule(node, script),
        Rule::declaration      => builders::declarations::process_declaration_rule(node, script),
        Rule::assignment       => builders::expressions::process_assignment_rule(node, script),
        Rule::expression       => builders::expressions::process_expression_rule(node, script),
        Rule::include          => builders::expressions::process_include_rule(node, script),
        Rule::import           => builders::expressions::process_import_rule(node, script),
        Rule::call_expression  => builders::expressions::process_call_expression_rule(node, script),
        _ => builders::utils::null_node(&node, script),
    }
}