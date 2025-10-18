//! ./reports/mod.rs
//! 
//! Module for defining and managing reports within the system.
//! This module provides structures and functionality for creating, accumulating, and displaying reports.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18

pub mod accumulator;
pub mod locations;
pub mod report;
pub mod macros;

pub use accumulator::*;
pub use report::*;
pub use locations::*;