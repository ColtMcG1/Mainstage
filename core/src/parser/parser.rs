//! ./parser/parser.rs
//!
//! Module for parsing scripts and generating AST nodes.
//! This module handles the parsing of scripts and extraction of relevant information.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

use crate::parser;
use crate::report;
use crate::reports;
use crate::scripts;

use pest::Parser;

/// Represents errors that can occur during parsing.
/// This enum is used to encapsulate various errors that can occur during the parsing process.
#[derive(Debug)]
pub enum ParserError {
    PestError(String),
    AstNodeError(String),
}

/// Parses the given script and returns an `AstParser` instance or a `ParserError` on failure.
/// This struct is responsible for parsing scripts and constructing the AST.
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
    pub root: parser::AstNode<'static>,
    pub script: scripts::Script,
}

impl AstParser {

    /// Parses the given script and returns a `pest::iterators::Pairs` instance or a `ParserError` on failure.
    /// This function uses the Pest parser to parse the script content.
    /// # Arguments
    /// * `script` - The script to be parsed.
    /// # Returns
    /// * `Ok(pest::iterators::Pairs)` if parsing is successful.
    fn parse_script<'a>(script: &'a scripts::Script) -> Result<pest::iterators::Pairs<'a, parser::Rule>, ParserError> {
        parser::MainstageParser::parse(parser::Rule::script, &script.content)
            .map_err(|e| {
                report!(
                    reports::Level::Error,
                    format!("Parsing error: {}", e),
                    Some("Pest".into()),
                    None,
                    None
                );
                ParserError::PestError(e.to_string())
            })
    }

    /// Constructs the AST from the given Pest parse pairs and script.
    /// This function builds the AST nodes from the parsed script content.
    /// # Arguments
    /// * `rules` - The Pest parse pairs.
    /// * `script` - The script being parsed.
    /// # Returns
    /// * `Ok(parser::AstNode)` if AST construction is successful.
    /// * `Err(ParserError)` if AST construction fails.
    fn construct_ast(
        rules: pest::iterators::Pairs<parser::Rule>,
        script: &scripts::Script,
    ) -> Result<parser::AstNode<'static>, ParserError> {
        parser::AstNode::new(rules, script)
            .map_err(|_| ParserError::AstNodeError("Failed to create AST node".to_string()))
            .map(|node| node.into_owned())
    }

    /// Creates a new `AstParser` instance by parsing the given script.
    /// This function combines parsing and AST construction to produce an `AstParser`.
    /// # Arguments
    /// * `script` - The script to be parsed.
    /// # Returns
    /// * `Ok(AstParser)` if parsing and AST construction are successful.
    pub fn new(script: &scripts::Script) -> Result<Self, ParserError> {
            let rules = Self::parse_script(script)?;
            let root = Self::construct_ast(rules, script)?;
            Ok(AstParser {
                root,
                script: script.clone(),
            })
        }

    /// Returns a reference to the root AST node.
    /// # Returns
    /// * A reference to the root `AstNode`.
    pub fn root(&self) -> &parser::AstNode<'static> {
        &self.root
    }

    /// Returns a reference to the script being parsed.
    /// # Returns
    /// * A reference to the script being parsed.
    pub fn script(&self) -> &scripts::Script {
        &self.script
    }
}