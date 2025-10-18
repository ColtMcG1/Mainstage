
#[derive(Debug, Clone, PartialEq)]
pub enum AstType {
    // --- Top Level ---
    Script,
    Include { path: String },
    Import { path: String, alias: String },

    // --- Scopes ---
    Workspace { name: String },
    Project { name: String },
    Stage { name: String },
    Task { name: String },

    // --- Expressions ---
    Assignment { left: String, right: String },

    // --- Declarations ---
    VariableDeclaration { name: String, value: Option<String> },

    // --- Values ---
    Identifier { name: String },
    ShellCommand { shell: String, command: String },
    String { value: String },
    Number { value: f64 },
    Boolean { value: bool },
    Null,
}