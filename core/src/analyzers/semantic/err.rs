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
        if let Some(loc) = &self.location {
            write!(f, "{} at {}", self.message, loc)
        } else if let Some(span) = &self.span {
            write!(f, "{} at span {:?}", self.message, span)
        } else {
            write!(f, "{}", self.message)
        }
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