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
- Added member access support in grammar, parser, and AST ([core/src/grammar.pest](core/src/grammar.pest), [core/src/parser/node.rs](core/src/parser/node.rs)).
- Added member access lowering in codegen ([core/src/codegen/lowering.rs](core/src/codegen/lowering.rs)).
- Added member access opcode in runtime ([core/src/runtime/opcode.rs](core/src/runtime/opcode.rs)).
- Added member access semantic checks in analyzer ([core/src/semantic/analyzer.rs](core/src/semantic/analyzer.rs)).


### 11/11/2025 - DAG refactor, modular lowering, Index/Pop opcodes
- Refactored DAG edge representation (IDs + EdgeKind) and added incoming adjacency ([core/src/acyclic/{edge.rs,node.rs,analyzer.rs}]).
- Added data dependency edges for Assignment RHS and Call arguments (EdgeKind::Data).
- Added CompactDag (index-based) + structural-only topo sort, cycle detection, DOT exporters.
- Implemented new bytecode ops Index (0x81) and Pop (0x82) with VM handlers ([core/src/runtime/{opcode.rs,vm.rs}], [core/src/codegen/{ir.rs,bytecode.rs}]).
- Lowering now emits Index for array subscripts and Pop for unused call results ([core/src/codegen/lowering/*]).
- Modularized lowering into discover.rs, expr.rs, stmt.rs, mod.rs (replacing monolithic lowering.rs).
- Updated IR dump and acyclic dump outputs ([cli/dump_ir.txt], [cli/dump_acyclic.txt]).

### 11/12/2025 - Init-on-first-reference, Ref values, and fixes
- Runtime/VM
  - Added init-on-first-reference: dereferencing a Ref triggers {scope}:{name}::init once.
  - Preallocated/auto-resizing globals; safe load/store helpers to avoid OOB panics.
  - Built local function index in VM for init lookups.
  - New opcode handler: LoadRefMember (resolves refs to globals).
- IR/Codegen
  - Added IRConst::Ref and IROpKind::LoadRefMember.
  - Workspace projects now stores Array<Ref{scope:"project", object:name}>.
  - Member access lowering prefers LoadRefMember for variables/expressions yielding refs.
- Parser
  - Fixed number parsing by trimming tokens before parse; added debug logging removal readiness.
- Sample
  - say(test1) prints project name; say(test1.test) now resolves (after auto-init).

### 11/15/2025 - Launch config, ref-based initialization cleanup
- Added VS Code launch configuration (lldb) for debugging CLI.
- Introduced IRConst::Ref and LoadRefMember usage refinements.
- Added workspace project reference array serialization as Ref entries.
- Updated sample to exercise ref auto-initialization.

### 11/16/2025 - Migration off old IR/bytecode to linear Op IR
- Removed legacy IRFunction / bytecode emitter / scheduler modules.
- Introduced unified IRProgram { ops, meta } with register-style Slot usage.
- Added lowering context (LowerCtx) tracking scopes, first-call initialization, member init.
- Reworked expression and statement lowering to emit direct Ops (LoadConst, MSet, etc.).
- Simplified entrypoint handling: [entrypoint] attribute drives initial Call + Halt.
- Updated parser node kinds (CallExpression -> Call, MemberAccess -> Member).
- Removed obsolete end-to-end bytecode test.

### 11/17/2025 - Enhanced scope/member resolution & expression lowering
- Added member read via MGet when initialized inside current scope.
- First-reference scope auto-init converted to emitting Call with Str func operand.
- Added semantic adjustments for scope call reference counting (reduces false “never referenced” warnings).
- Added runtime string Display implementations for RTValue.

### 11/18/2025 - Loop model, Forto (for-to) syntax, control flow & VM improvements
- Grammar: added forto_stmt (for i=0 to N) replacing generic parentheses for-ranges.
- Parser: new AstType::Forto { init, limt, body }; removed old For triple (init/cond/step).
- Lowering: distinct lowering paths for Forin and Forto, emitting Inc op for counters.
- Added Halt, Inc, Dec ops; meta analysis updated to track them.
- Runtime VM: 
  - Proper array growth (ensure_len), in-place mutation via get_mut.
  - Call now supports dynamic resolution of function/scope labels (func.<name> / scope.<name>).
  - Return propagates value to caller’s target slot; Halt terminates cleanly.
  - Added safe string concatenation path in Add when operands are (partially) strings.
- Entry selection now respects [entrypoint] across any scope (project, workspace, etc.).
- Refined semantic call analysis (scope calls treated as non-value procedures unless task-like).