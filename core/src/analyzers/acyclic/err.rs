//! Error type for the acyclic analyzer.
//!
//! `AcyclicError` is returned by the acyclic pass when cycles are detected in
//! the stage call graph. It implements `MainstageErrorExt` for unified
//! diagnostic reporting.

use crate::location::{Location, Span};
use crate::error::{Level, MainstageErrorExt};

#[derive(Debug, Clone)]
pub struct AcyclicError {
    level: Level,
    message: String,
    issuer: String,
    location: Option<Location>,
    span: Option<Span>,
}

impl AcyclicError {
    pub fn with(
        level: Level,
        message: String,
        issuer: String,
        location: Option<Location>,
        span: Option<Span>,
    ) -> Self {
        AcyclicError {
            level,
            message,
            issuer,
            location,
            span,
        }
    }
}

impl std::fmt::Display for AcyclicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Build a compact, consistent diagnostic line similar to the global Debug
        // representation used for MainstageErrorExt, but human-friendly.
        let loc_str = match &self.location {
            Some(loc) => format!("{}:{}:{}", loc.file, loc.line, loc.column),
            None => "unknown".to_string(),
        };

        let span_str = match &self.span {
            Some(span) => span.to_string(),
            None => "span:none".to_string(),
        };

        write!(
            f,
            "MAINSTAGE | {} | {} | {} | {} | {}",
            self.level, loc_str, self.issuer, span_str, self.message
        )
    }
}

impl std::error::Error for AcyclicError {}

impl MainstageErrorExt for AcyclicError {
    fn level(&self) -> Level {
        self.level
    }

    fn message(&self) -> String {
        self.message.clone()
    }

    fn issuer(&self) -> String {
        self.issuer.clone()
    }

    fn span(&self) -> Option<Span> {
        self.span.clone()
    }

    fn location(&self) -> Option<Location> {
        self.location.clone()
    }
}