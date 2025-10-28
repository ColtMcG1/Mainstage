# TODO List for where I left off

## Semantic checks (implement in semantic analyzer / type system / symbol table)

- [ ] Function/task parameter count — error at call-site resolution (semantic).
- [ ] Return type checks — error in function/task body analysis (control-flow + types).
- [ ] Constant reassignment — error on symbol redefinition if marked const.
- [ ] Scope leakage — error/warning when resolving a symbol outside its valid scope.
- [ ] Parameter type inference — semantic/type-inference subsystem.

## Acyclic / dependency (implement in DAG / dependency analysis phase)

- [ ] Hot-path / high-reference ordering effects — derive from reference counts but analyze in dependency graph for scheduling/optimization.