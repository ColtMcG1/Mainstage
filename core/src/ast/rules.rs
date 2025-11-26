use pest_derive::Parser;

use crate::location;

#[derive(Parser)]
#[grammar = "grammar.pest"]

pub struct RulesParser;

pub(crate) fn fetch_next_pair<'a>(
    pairs: &mut pest::iterators::Pairs<'a, Rule>,
    location: &Option<location::Location>,
    span: &Option<location::Span>,
) -> Result<pest::iterators::Pair<'a, Rule>, Box<dyn crate::ast::MainstageErrorExt>> {
    match pairs.next() {
        Some(pair) => Ok(pair),
        None => Err(Box::<dyn crate::ast::MainstageErrorExt>::from(Box::new(
            crate::ast::err::SyntaxError::with(
                crate::Level::Error,
                "Expected more inner pairs but found none.".into(),
                "mainstage.ast.rules.get_next_inner_pair".into(),
                location.clone(),
                span.clone(),
            ),
        ))),
    }
}

pub(crate) fn get_data_from_rule<'a>(
    rule: &pest::iterators::Pair<'a, Rule>,
    script: &crate::script::Script,
) -> (
    pest::iterators::Pairs<'a, Rule>,
    Option<crate::location::Location>,
    Option<crate::location::Span>,
) {
    let inner_rules = rule.clone().into_inner();
    let span = get_span_from_pair(&rule, script);
    let location = get_location_from_pair(&rule, script);
    (inner_rules, location, span)
}

pub fn get_location_from_pair(
    rule: &pest::iterators::Pair<Rule>,
    script: &crate::script::Script,
) -> Option<crate::location::Location> {
    let span = rule.as_span();
    Some(crate::location::Location {
        file: script.name.clone(),
        line: span.start_pos().line_col().0,
        column: span.start_pos().line_col().1,
    })
}

pub fn get_span_from_pair(
    rule: &pest::iterators::Pair<Rule>,
    script: &crate::script::Script,
) -> Option<crate::location::Span> {
    let span = rule.as_span();
    Some(crate::location::Span {
        start: crate::location::Location {
            file: script.name.clone(),
            line: span.start_pos().line_col().0,
            column: span.start_pos().line_col().1,
        },
        end: crate::location::Location {
            file: script.name.clone(),
            line: span.end_pos().line_col().0,
            column: span.end_pos().line_col().1,
        },
    })
}
