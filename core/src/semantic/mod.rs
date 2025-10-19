//! ./semantic/mod.rs
//! 
//! Module for various semantic analysis functionalities.
//! This module serves as a container for different semantic analysis components and functionalities.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

pub mod analyzer;
pub mod symbol;

pub use analyzer::*;
pub use symbol::*;