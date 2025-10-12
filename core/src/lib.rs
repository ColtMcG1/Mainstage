/// Core library for Mainstage application
/// author: Colton McGraw
/// date: October 10th, 2025
/// description: This library provides core functionalities for the Mainstage application,
/// including script parsing, error reporting, and utility functions.
/// license: TBD
pub mod reports;

pub use reports::{ErrorCode, Location, Report, ReportCollector, Severity, Span, SpanType};

use crate::reports::*;

/// Represents a script with its path and content
#[derive(Clone)]
pub struct Script {
    path: String,
    content: String,
}

impl Script {
    pub fn new(path: &str, collector: &mut ReportCollector) -> Self {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        if content.is_empty() {
            collector.push(Report::fatal(
                "Failed to read script content",
                Some(Location {
                    file: path.to_string(),
                    line: 1,
                    column: 1,
                    span: Span {
                        start: 0,
                        end: 0,
                        span_type: SpanType::File,
                    },
                }),
                Some(E_IO),
                Some("Check that your file exists and is readable.".to_string()),
                None,
            ));
        }
        Script {
            path: path.to_string(),
            content,
        }
    }
    pub fn get_path(&self) -> &str {
        &self.path
    }
    pub fn get_content(&self) -> &str {
        &self.content
    }
}

/// Expand a script by resolving includes and macros
pub fn expand_script(
    script: &Script,
    collector: &mut ReportCollector
) -> Result<Script, ReportCollector> {

    Ok(script.clone())
}

/// Parse a script into an AST
pub fn parse_script(
    script: &Script,
    collector: &mut ReportCollector,
) -> Result<Script, ReportCollector> {

    Ok(script.clone())
}

/// Analyze the AST for semantic correctness and type checking
/// Returns an Intermediate Representation (IR) of the script
pub fn analyze_script(
    script: &Script,
    collector: &mut ReportCollector,
) -> Result<Script, ReportCollector> {

    Ok(script.clone())
}

/// Resolve dependencies and references in the script
/// Returns a resolved script with all dependencies included
pub fn resolve_script(
    script: &Script,
    collector: &mut ReportCollector,
) -> Result<Script, ReportCollector> {

    Ok(script.clone())
}

/// Create a Directed Acyclic Graph (DAG) of script tasks for execution
/// Returns a DAG representation of the script
pub fn make_script_dag(
    script: &Script,
    collector: &mut ReportCollector,
) -> Result<Script, ReportCollector> {

    Ok(script.clone())
}

/// Plan the execution of the script based on the DAG
/// Returns a planned script ready for execution
pub fn plan_script(
    script: &Script,
    collector: &mut ReportCollector,
) -> Result<Script, ReportCollector> {

    Ok(script.clone())
}

/// Execute the script and return the results
/// Returns the executed script with results
pub fn execute_script(
    script: &Script,
    collector: &mut ReportCollector,
) -> Result<Script, ReportCollector> {

    Ok(script.clone())
}