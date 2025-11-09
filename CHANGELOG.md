# CHANGELOG

## v0.1.0 - Pre-Alpha Release

### 10/11/2025 - Setup
- Project initialization.
- Added root [.gitignore](.gitignore) and placeholder [README.md](README.md).

### 10/18/2025 - Grammar, Parser, AST, semantic analyzer, expander, media, LICENSE, & CLI
- Initial grammar ([core/src/grammar.pest](core/src/grammar.pest)).
- Basic AST node structure ([core/src/parser/node.rs](core/src/parser/node.rs)).
- Initial parser integration.
- Added report system ([core/src/reports/mod.rs](core/src/reports/mod.rs), [core/src/reports/report.rs](core/src/reports/report.rs)).
- Added script/map utilities ([core/src/scripts/script.rs](core/src/scripts/script.rs), [core/src/scripts/map.rs](core/src/scripts/map.rs)).
- Added LICENSE ([LICENSE.md](LICENSE.md)).
- Added media assets (logos) under [media/](media).
- Added CLI crate ([cli/Cargo.toml](cli/Cargo.toml)).

### 10/19/2025 - Expanded AST node types
- Added support for arrays, booleans, numbers, shell commands in AST ([core/src/parser/node.rs](core/src/parser/node.rs)).
- Extended grammar for arrays, shell strings, arguments ([core/src/grammar.pest](core/src/grammar.pest)).
- Workspace / project / stage / task bodies populated with statements.

### 10/21/2025 - Expanded semantic analyzer & added TODO file
- Added semantic analyzer scaffolding ([core/src/semantic/analyzer.rs](core/src/semantic/analyzer.rs)).
- Added symbol table & symbol kinds ([core/src/semantic/symbol.rs](core/src/semantic/symbol.rs)).
- Introduced [TODO.md](TODO.md) for planned semantic checks.

### 10/25/2025 - Setup directed acyclic graph
- Added DAG analyzer output dump ([cli/dump_acyclic.txt](cli/dump_acyclic.txt)).
- Integrated acyclic analysis stage into pipeline ([core/src/lib.rs](core/src/lib.rs)).

### 10/27/2025 - Update TODO & semantic analyzer
- Refined TODO items (semantic focus).
- Added lifetime handling adjustments for AST and parser.
- Improved location & span propagation in AST ([core/src/parser/node.rs](core/src/parser/node.rs)).
- Added license headers across modules.

### 11/08/2025 - Setup intermediate code generator & runtime. Updated semantic analyzer.
- Introduced IR module ([core/src/codegen/ir.rs](core/src/codegen/ir.rs)).
- Added lowering pass ([core/src/codegen/lowering.rs](core/src/codegen/lowering.rs)).
- Added bytecode emitter ([core/src/codegen/bytecode.rs](core/src/codegen/bytecode.rs)).
- Added scheduling utilities ([core/src/codegen/scheduler.rs](core/src/codegen/scheduler.rs)).
- Added basic runtime opcodes ([core/src/runtime/opcode.rs](core/src/runtime/opcode.rs)).
- Added end-to-end test ([cli/tests/e2e.rs](cli/tests/e2e.rs)).
- Added pipeline IR generation & execution hooks ([core/src/lib.rs](core/src/lib.rs)).
- Expanded semantic analyzer warnings and parameter handling.

### 11/09/2025 - Expand code generator & runtime. Update grammar, parser, & semantic analyzer.
- Fixed codegen ordering & entrypoint resolution (main dispatch).
- Added plain function name index for stage/task calls ([core/src/codegen/ir.rs](core/src/codegen/ir.rs)).
- Added Say/Read/Write op variants ([core/src/runtime/opcode.rs](core/src/runtime/opcode.rs), [core/src/codegen/bytecode.rs](core/src/codegen/bytecode.rs)).
- Enhanced lowering to schedule scope bodies & emit globals in deterministic order ([core/src/codegen/lowering.rs](core/src/codegen/lowering.rs)).
- Added value/type handling in semantic analyzer (empty body warnings, duplicate parameter errors) ([core/src/semantic/analyzer.rs](core/src/semantic/analyzer.rs)).
- Grammar simplifications (removed expression_statement, unified arrays/arguments) ([core/src/grammar.pest](core/src/grammar.pest)).
- Added function name retention in bytecode for VM dispatch.
- Added CLI dump stages for loader/parser/IR/DAG ([core/src/lib.rs](core/src/lib.rs), [cli/src/main.rs](cli/src/main.rs)).
- Fixed date formatting & improved diagnostics formatting in parser & reports.
- Added CHANGELOG.md ([CHANGELOG.md](CHANGELOG.md)) for tracking and compiling the changes made to the project.