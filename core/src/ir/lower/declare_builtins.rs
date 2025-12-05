//! file: core/src/ir/lower/declare_builtins.rs
//! description: ensure a small set of runtime builtin functions are declared.
//!
//! Emits declarations for commonly-used host functions (`say`, `read`,
//! `write`, etc.) so calls lower to the expected call forms instead of
//! falling back to null-producing operations.
//!
use crate::ir::{IrModule, lower::LoweringContext};
pub(crate) fn declare_builtin_functions(ir_mod: &mut IrModule, ctx: &mut LoweringContext) {
    let builtins = vec!["say", "ask", "read", "write", "time", "random", "fmt"];
    for name in builtins {
        if ctx.symbols.get(name).is_none() {
            let id = ir_mod.declare_function(name);
            ctx.symbols.insert(name.to_string(), id);
        }
    }
}
