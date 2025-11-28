use crate::location::{Location, Span};
use crate::error::{Level, MainstageErrorExt};

#[derive(Debug, Clone)]
pub struct SemanticError {
    level: Level,
    message: String,
    issuer: String,
    location: Option<Location>,
    span: Option<Span>,
}

impl SemanticError {
    pub fn with(
        level: Level,
        message: String,
        issuer: String,
        location: Option<Location>,
        span: Option<Span>,
    ) -> Self {
        SemanticError {
            level,
            message,
            issuer,
            location,
            span,
        }
    }
}

impl std::fmt::Display for SemanticError {
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

impl std::error::Error for SemanticError {}

impl MainstageErrorExt for SemanticError {
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