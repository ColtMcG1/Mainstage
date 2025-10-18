//! ./parser/mod.rs
//! 
//! Module for parsing scripts and generating AST nodes.
//! This module handles the parsing of scripts and extraction of relevant information.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-14

pub mod types;
pub mod node;
pub mod parser;

pub use types::*;
pub use node::*;
pub use parser::*;