# TODO List for where I left off

## Semantic checks (implement in semantic analyzer / type system / symbol table)

- Duplicate parameter names — error at parameter insertion (symbol table).
- Unused parameters — warning after scope analysis (reference counts).
- Shadowing detection — warning/error during insertion (symbol table lookup).
- Reserved keyword usage — error when defining symbols (lexer/semantic).
- Type compatibility in assignments — error in infer_type / assignment handling.
- Function/task parameter count — error at call-site resolution (semantic).
- Return type checks — error in function/task body analysis (control-flow + types).
- Constant reassignment — error on symbol redefinition if marked const.
- Scope leakage — error/warning when resolving a symbol outside its valid scope.
- Unreachable code — semantic/control-flow analysis (CFG), usually a warning.
- Parameter type inference — semantic/type-inference subsystem.
- Task/stage name uniqueness — error when inserting global declarations (symbol table).

## Acyclic / dependency (implement in DAG / dependency analysis phase)

- Cyclic reference detection — build dependency graph and detect cycles (DAG).
- Hot-path / high-reference ordering effects — derive from reference counts but analyze in dependency graph for scheduling/optimization.