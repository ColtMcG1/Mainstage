use super::node::AstNode;

#[derive(Debug, Clone, PartialEq)]
pub enum AstNodeKind {
    Script { body: Vec<AstNode> },
    Import { module: String },
    Include { file: String },

    Statement,
    Arguments { args: Vec<AstNode> },

    Workspace { name: String, body: Box<AstNode> },
    Project { name: String, body: Box<AstNode> },
    Stage { name: String, args: Option<Box<AstNode>>, body: Box<AstNode> },

    Block { statements: Vec<AstNode> },

    If { condition: Box<AstNode>, body: Box<AstNode> },
    IfElse { condition: Box<AstNode>, if_body: Box<AstNode>, else_body: Box<AstNode> },

    ForIn { iterator: String, iterable: Box<AstNode>, body: Box<AstNode> },
    ForTo { initializer: Box<AstNode>, limit: Box<AstNode>, body: Box<AstNode> },
    While { condition: Box<AstNode>, body: Box<AstNode> },

    UnaryOp { op: String, expr: Box<AstNode> },
    BinaryOp { left: Box<AstNode>, op: String, right: Box<AstNode> },
    Assignment { target: Box<AstNode>, value: Box<AstNode> },

    Command { name: String, arg: String },
    Call { callee: Box<AstNode>, args: Vec<AstNode> },
    Return { value: Option<Box<AstNode>> },

    Identifier { name: String },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Bool { value: bool },
    List { elements: Vec<AstNode> },
    Null,
}