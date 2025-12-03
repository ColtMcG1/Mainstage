# Pest Grammar for the Mainstage scripting language

## Lexical rules

- All whitespaces are ignored (except within strings).
- Single-line comments start with `//` and continue to the end of the line. There are also ignored by the parser.

```pest
// --- Lexical / Skipping ---
WHITESPACE = _{ " " | "\t" | "\r" | "\n" | COMMENT }
COMMENT    = _{ "//" ~ (!"\n" ~ ANY)* ~ ("\n" | EOI) }
```

## Grammar rules

- The top-level structure is a `script`, which consists of multiple `item`s and is bounded by the start and end of the input stream (SOI and EOI).
- An `item` can be either a [`declaration`](#declarations) (like `workspace`, `project`, or `stage`) or a [`statement`](#statements).

```pest
// --- Top-Level ---
script = { SOI ~ item* ~ EOI }

// Items are either declarations or statements (where statements can be blocks)
item = { declaration | statement }
```

## Statements

- Statements are broken down into various types, including terminated statements, loops, conditionals, and blocks.
- Terminated statements are all statements that end with a semicolon.
- Loops and conditionals do not require a trailing semicolon, as their bodies are [`block`](#block)s.

```pest
// --- Statements ---
statement = { terminated_statement | loop_stmt | conditional_stmt | block }

terminated_statement = {
    return_stmt
  | include_stmt
  | import_stmt
  | assignment_stmt
  | expression_stmt
}

return_stmt     = { "return" ~ expression ~ ";" }
include_stmt    = { "include" ~ string ~ ";" }
import_stmt     = { "import" ~ string ~ "as" ~ identifier ~ ";" }
expression_stmt = { expression ~ ";" }

// Replace existing rules that used bare "="
assignment_stmt = { identifier ~ assign_op ~ expression ~ ";" }
```

## Block

- A `block` is defined as a series of [`statement`](#statements)s enclosed within curly braces `{}`.
- Blocks are used in declarations, loops, and conditionals to group multiple statements together.
- Any declarations within a block are scoped to that block. Only top-level declarations (workspaces, projects, stages) are globally scoped.
- Blocks do not require a trailing semicolon and can be nested. Empty blocks will emit a warning during parsing and are discouraged in practice.

```pest
// --- Blocks ---
block = { "{" ~ statement* ~ "}" }
```

## Declarations

- Declarations define high-level constructs like which are object like.
- Declarations do not require a trailing semicolon.
- Each declaration consists of optional attributes, a keyword, an identifier, and a body (which is a block for workspaces and projects, and a parameterized block for stages).
- Declarations can have optional attributes enclosed in square brackets `[]` prior to the keyword.
- The `stage` declaration can take parameters within parentheses `()` before the block body. It can also return values via `return` statements inside its block.

```pest
// --- Declarations (no trailing semicolon) ---
declaration   = { workspace_decl | project_decl | stage_decl }

workspace_decl = { attributes? ~ "workspace" ~ identifier ~ block }
project_decl   = { attributes? ~ "project"   ~ identifier ~ block }
stage_decl     = { attributes? ~ "stage"     ~ identifier ~ "(" ~ arguments? ~ ")" ~ block }
```

## Conditionals and Loops

- Conditionals include `if`, `if-else`, and ternary expressions.
- Loops include `for-in`, `for-to`, and `while` loops.

- Conditionals and loops do not require a trailing semicolon, and their bodies must be [`blocks`](#block).
- The `for-to` loop header uses an [`assignment_expr`](#expressions) to allow initialization of the loop variable.

```pest
// --- Conditionals (no trailing semicolon; body must be a block) ---
conditional_stmt = { if_else_stmt | if_stmt | tenary_stmt }
if_stmt        = { "if" ~ expression ~ block }
if_else_stmt    = { "if" ~ expression ~ block ~ "else" ~ block }
tenary_stmt    = { expression ~ "?" ~ expression ~ ":" ~ expression ~ ";" }

// --- Loops (no trailing semicolon; body must be a block) ---
loop_stmt    = { for_in_stmt | for_to_stmt | while_stmt }
for_in_stmt   = { "for" ~ identifier ~ "in" ~ expression ~ block }
for_to_stmt   = { "for" ~ assignment_expr ~ "to" ~ expression ~ block }
while_stmt   = { "while" ~ expression ~ block }
```

## Operators

- Various operators are defined for assignments, equality checks, relational comparisons, arithmetic operations, and unary operations.

```pest
// Add operator set
assign_op = { "=" | "+=" | "-=" | "*=" | "/=" | "%=" }
eq_op    = { "==" | "!=" }
rel_op   = { "<=" | ">=" | "<" | ">" }
add_op   = { "+" | "-" }
mul_op   = { "*" | "/" | "%" }
unary_op = { "++" | "--" | "+" | "-" | "!" }
```

## Expressions

- Expressions support operator precedence and associativity, including equality, relational, additive, multiplicative, unary, and postfix operations.
- Postfix operations include function calls, member access, indexing, and postfix increments/decrements.
- Primary expressions include values, identifiers, and parenthesized expressions.
- An `assignment_expr` is defined for use in the `for-to` loop header to allow initialization of the loop variable.

```pest
// --- Expressions ---
// Make calls/members/index postfix ops so chaining works: obj.fn(a).x[i]++.
expression                = { equality_expression }
equality_expression       = { relational_expression ~ (eq_op  ~ relational_expression)* }
relational_expression     = { additive_expression   ~ (rel_op ~ additive_expression)* }
additive_expression       = { multiplicative_expression ~ (add_op ~ multiplicative_expression)* }
multiplicative_expression = { unary_expression      ~ (mul_op ~ unary_expression)* }
unary_expression          = { (unary_op)* ~ postfix_expression }

postfix_expression = { primary_expression ~ (postfix_op)* }
postfix_op = {
      "(" ~ arguments? ~ ")"        // call
    | "." ~ identifier              // member
    | "[" ~ expression ~ "]"        // index
    | "++"                          // postfix inc
    | "--"                          // postfix dec
}

assignment_expr = { identifier ~ assign_op ~ expression }  // for forto header
primary_expression = { value | identifier | "(" ~ expression ~ ")" }
```

## Arguments

- Arguments are defined as a comma-separated list of [`expression`](#expressions).
- A trailing comma is allowed.
- Each argument is a single [`expression`](#expressions).

```pest
// --- Arguments ---
parameter  = { expression }
arguments  = { parameter ~ ("," ~ parameter)* ~ ","? }   // trailing comma ok
```

## Attributes

- Attributes are defined as a comma-separated list of identifiers enclosed in square brackets `[]`.
- A trailing comma is allowed.
- Each attribute is a single identifier.

```pest
// --- Attributes ---
attribute  = { identifier }
attributes = { "[" ~ attribute ~ ("," ~ attribute)* ~ ","? ~ "]" }
```

## Values

- Values include arrays, shell strings, regular strings, booleans, numbers, and null.
- An array is defined as a comma-separated list of [`expression`](#expressions) enclosed in square brackets `[]`.
- A shell string is defined as a string command prefixed by a shell identifier (e.g., `sh`, `bash`, `zsh`, `pwsh`, `cmd`).
- A string is defined as a sequence of characters enclosed in double quotes `""`.

```pest
// --- Values ---
value        = { array | shell_string | string | boolean | number | null }
array        = { "[" ~ (expression ~ ("," ~ expression)*)? ~ "]" }
boolean      = { "true" | "false" }
number       = { ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? }
string       = { "\"" ~ (!"\"" ~ ANY)* ~ "\"" }
shell_string = { shell_prefix ~ string }
shell_prefix = { "sh" | "bash" | "zsh" | "pwsh" | "cmd" }
null         = { "null" }

// --- Identifiers ---
identifier = @{ (ASCII_ALPHA | "_") ~ (ASCII_ALPHANUMERIC | "_")* }
```

---
