/// Core library for Mainstage application
/// author: Colton McGraw
/// date: October 10th, 2025
/// description: This library provides core functionalities for the Mainstage application,
/// including script parsing, error reporting, and utility functions.
/// license: TBD

pub mod reports;

pub use reports::{Report, ReportCollector, Severity, ErrorCode, Location, Span, SpanType};

use crate::reports::E_IO;

#[derive(Clone)]
pub struct Script {
    path: String,
    content: String,
}

impl Script {
    pub fn new(path: &str, collector: &mut ReportCollector) -> Self {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        if content.is_empty() {
            collector.push(Report::new(
                "Failed to read script content",
                Severity::Warning,
                Some(Location {
                    file: path.to_string(),
                    line: 1,
                    column: 1,
                    span: Span { start: 0, end: 0, span_type: SpanType::File },
                }),
                None,
                None,
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

pub fn parse_script(script: &Script, collector: &mut ReportCollector) -> Result<Script, ReportCollector> {
    // Placeholder for script parsing logic

    std::thread::sleep(std::time::Duration::from_secs(1));

    if true {
        let report = Report::fatal(
            "Parsing failed due to syntax error",
            Some(Location {
                file: script.get_path().to_string(),
                line: 10,
                column: 5,
                span: Span { start: 100, end: 105, span_type: SpanType::Line },
            }),
            Some(E_IO),
            None,
            None,
        );
        collector.push(report.clone());
    }

    Ok(script.clone())
}