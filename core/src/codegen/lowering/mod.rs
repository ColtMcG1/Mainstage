//! ./codegen/lowering/mod.rs
//! Thin facade for lowering. Implementation is split across submodules.

mod expr;
mod stmt;
mod discover;

// Re-export the public API
pub use discover::lower_ast_to_ir;