use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Integer,
    Float,
    String,
    Boolean,
    Void,
    Null,
    Object,
    Array,
    Dynamic,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Kind::Integer => "Integer",
            Kind::Float => "Float",
            Kind::String => "String",
            Kind::Boolean => "Boolean",
            Kind::Void => "Void",
            Kind::Null => "Null",
            Kind::Object => "Object",
            Kind::Array => "Array",
            Kind::Dynamic => "Dynamic",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    Expression,
    Coerced,
    Unknown,
}

impl Default for Origin {
    fn default() -> Self {
        Origin::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredKind {
    pub kind: Kind,
    pub origin: Origin,
    pub location: Option<crate::location::Location>,
    pub span: Option<crate::location::Span>,
}

impl Default for InferredKind {
    fn default() -> Self {
        InferredKind {
            kind: Kind::Dynamic,
            origin: Origin::Unknown,
            location: None,
            span: None,
        }
    }
}

impl InferredKind {
    pub fn new(
        kind: Kind,
        origin: Origin,
        location: Option<crate::location::Location>,
        span: Option<crate::location::Span>,
    ) -> Self {
        InferredKind {
            kind,
            origin,
            location,
            span,
        }
    }

    // Convenience constructors
    pub fn integer() -> Self {
        Self::new(Kind::Integer, Origin::Expression, None, None)
    }
    pub fn float() -> Self {
        Self::new(Kind::Float, Origin::Expression, None, None)
    }
    pub fn string() -> Self {
        Self::new(Kind::String, Origin::Expression, None, None)
    }
    pub fn boolean() -> Self {
        Self::new(Kind::Boolean, Origin::Expression, None, None)
    }
    pub fn dynamic() -> Self {
        Self::default()
    }

    // Builder-style modifiers useful in analysis passes
    pub fn with_origin(mut self, origin: Origin) -> Self {
        self.origin = origin;
        self
    }
    pub fn with_location(mut self, loc: crate::location::Location) -> Self {
        self.location = Some(loc);
        self
    }
    pub fn with_span(mut self, span: crate::location::Span) -> Self {
        self.span = Some(span);
        self
    }

    // Predicates
    pub fn is_numeric(&self) -> bool {
        matches!(self.kind, Kind::Integer | Kind::Float)
    }
    pub fn is_dynamic(&self) -> bool {
        matches!(self.kind, Kind::Dynamic)
    }
    pub fn is_null(&self) -> bool {
        matches!(self.kind, Kind::Null)
    }

    // Compatibility test: true if values of `other` can be used where `self` is expected.
    // Dynamic and Null are treated permissively; caller can tighten rules if needed.
    pub fn is_compatible_with(&self, other: &InferredKind) -> bool {
        if self.is_dynamic() || other.is_dynamic() {
            return true;
        }
        if self.kind == other.kind {
            return true;
        }
        // allow Null to be used with non-primitive containers or Dynamic
        if other.is_null() {
            return true;
        }
        // numeric coercion allowed (Integer -> Float)
        if matches!(self.kind, Kind::Float) && matches!(other.kind, Kind::Integer) {
            return true;
        }
        false
    }

    // Return the unified/coerced kind for two operands (used for binary arithmetic, etc.)
    // Simple rules:
    // - same => same
    // - Integer + Float => Float
    // - anything with Dynamic => Dynamic
    // - if incompatible => Dynamic (caller may treat as error)
    pub fn unify(&self, other: &InferredKind) -> InferredKind {
        use Kind::*;
        if self.kind == other.kind {
            return InferredKind {
                kind: self.kind.clone(),
                origin: Origin::Coerced,
                location: self.location.clone().or(other.location.clone()),
                span: self.span.clone().or(other.span.clone()),
            };
        }
        if self.is_dynamic() || other.is_dynamic() {
            return InferredKind::dynamic();
        }
        match (&self.kind, &other.kind) {
            (Float, Integer) | (Integer, Float) => InferredKind::new(
                Float,
                Origin::Coerced,
                self.location.clone().or(other.location.clone()),
                self.span.clone().or(other.span.clone()),
            ),
            (Null, k) | (k, Null) => InferredKind::new(
                k.clone(),
                Origin::Coerced,
                self.location.clone().or(other.location.clone()),
                self.span.clone().or(other.span.clone()),
            ),
            _ => InferredKind::dynamic(),
        }
    }
}

impl fmt::Display for InferredKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let origin = match &self.origin {
            Origin::Expression => "inferred",
            Origin::Coerced => "coerced",
            Origin::Unknown => "unknown",
        };
        write!(f, "{} ({})", self.kind, origin)
    }
}