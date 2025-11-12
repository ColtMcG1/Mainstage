//! ./parser/mod.rs
//! 
//! Module for parsing scripts and generating AST nodes.
//! This module handles the parsing of scripts and extraction of relevant information.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-14
//! license: See LICENSE file in the root directory

pub mod types;
pub mod parser;
pub mod expander;
pub mod attributes;
pub mod driver;      // pest parser + Rule
pub mod ast;         // AstNode struct + common helpers
pub mod builders;    // rule-specific processors

pub use types::*;
pub use parser::*;
pub use attributes::*;
pub use driver::MainstageParser;
pub use ast::AstNode;