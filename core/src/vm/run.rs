//! file: core/src/vm/run.rs
//! description: executor shim
//!
//! This module re-exports the actual bytecode runner implementation so the
//! top-level VM API can call `run::run_bytecode`. The real executor lives in
//! `vm::exec`.

pub(crate) use crate::vm::exec::run_bytecode;
