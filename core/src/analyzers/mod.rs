//! Analyzer entrypoints and re-exports.
//!
//! This crate-level module exposes the analyzer passes (`semantic`, `acyclic`)
//! and the shared `output` types so callers can run analysis and consume the
//! produced summaries.

pub mod semantic;
pub mod acyclic;
pub mod output;

pub use semantic::analyze_semantic_rules;
pub use acyclic::analyze_acyclic_rules;
pub use output::AnalyzerOutput;