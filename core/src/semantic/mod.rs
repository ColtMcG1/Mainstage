//! ./semantic/mod.rs
//!
//! Central module for semantic analysis. This file wires submodules and
//! re-exports common types so existing imports continue to work.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

pub mod symbol;      // Symbol + kinds
pub mod analyzer;    // SemanticAnalyzer
pub mod reserved;
pub mod builtin;

// Newly active modules:
pub mod types;       // InferredType, SymbolType
pub mod inference;   // infer_* helpers
pub mod scope;       // SymbolTable

// Re-export for callers
pub use analyzer::SemanticAnalyzer;
pub use symbol::{Symbol, SymbolKind, SymbolScope};
pub use scope::SymbolTable;
pub use types::{InferredType, SymbolType};