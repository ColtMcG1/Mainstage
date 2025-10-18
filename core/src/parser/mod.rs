//! This module handles the parsing of scripts and extraction of relevant information.
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-14

pub mod types;
pub mod node;

use node::AstNode;

use crate::report;
use crate::reports;
use crate::scripts::script::Script;

use pest::Parser;

/// The main parser struct that holds the root AST node and an accumulator for reports.
/// author: Colton McGraw <https://github.com/ColtMcG1>
/// date: 2025-10-14
/// # Examples
/// ```
/// use mainstage::parser::AstParser;
/// use mainstage::scripts::Script;
///
/// let script = Script::new("test", "echo hello");
/// let parser = AstParser::new(&script);
/// ```
#[derive(Debug, Clone)]
pub struct AstParser {
    pub root: AstNode,
}

impl AstParser {
    /// Parses the given script and returns an `AstParser` instance or an `Accumulator` of reports on failure.
    /// # Arguments
    /// * `script` - The script to be parsed.
    /// # Returns
    /// * `Ok(AstParser)` if parsing is successful.
    pub fn new(script: &Script) -> Result<Self, ()> {
        match node::MainstageParser::parse(node::Rule::script, &script.content) {
            Ok(rules) => {
                return AstNode::new(rules, script)
                    .map(|root| AstParser {
                        root,
                    })
                    .map_err(|_| { });
            }
            Err(e) => {
                report!(
                    reports::Level::Error,
                    format!("Parsing error: {}", e),
                    Some("Pest".into()),
                    None,
                    None
                );
                return Err(());
            }
        }
    }
}
