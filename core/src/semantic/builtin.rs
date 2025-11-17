use std::collections::HashSet;

pub struct Builtins {
    value_returning: HashSet<&'static str>,
    all: HashSet<&'static str>,
}

impl Builtins {
    pub fn new() -> Self {
        let all: HashSet<_> = ["say", "ask", "read", "write"].into_iter().collect();
        let value_returning: HashSet<_> = ["ask", "read"].into_iter().collect();
        Self { all, value_returning }
    }
    pub fn is(&self, name: &str) -> bool { self.all.contains(name) }
    pub fn returns_value(&self, name: &str) -> bool { self.value_returning.contains(name) }
}