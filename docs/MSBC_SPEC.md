# MSBC Bytecode Specification

This document describes the MSBC (Mainstage ByteCode) binary format used by the `mainstage` project. The VM implementation that reads this format is in `core/src/vm.rs`.

All integer fields are little-endian unless otherwise noted.

## File layout

- 4 bytes: ASCII magic `MSBC` (0x4D 0x53 0x42 0x43)
- 4 bytes: version (u32). Current supported version: `1`.
- 4 bytes: op_count (u32) — number of encoded ops that follow.
- `op_count` op records: each op consists of a 1-byte opcode followed by opcode-specific payload.

Notes:

- The op_count enumerates op records; some ops carry embedded variable-length payloads (strings, nested values, arrays/objects).
- The VM parser builds label maps from Label ops (see Label opcode) and resolves CallLabel operands by label name/ordinal.

## Value serialization (used by LConst and by nested constants in the stream)

Each Value starts with a 1-byte tag followed by tag-specific payload.

Tags and payloads:

- `0x01` — Int
  - payload: 8 bytes (u64 little-endian). Interpreted as i64 by the VM.
- `0x02` — Float
  - payload: 8 bytes (u64 bit pattern representing f64)
- `0x03` — Bool
  - payload: 1 byte (0x00 = false, otherwise true)
- `0x04` — Str
  - payload: 4-byte length `N` (u32), followed by `N` UTF-8 bytes
- `0x05` — Symbol
  - payload: 4-byte length `N` (u32), followed by `N` UTF-8 bytes
  - `Symbol` values are used for host function names (e.g., `"say"`) and other symbolic references
- `0x06` — Array
  - payload: 4-byte length `L` (u32), followed by `L` serialized `Value`s
- `0x08` — Object
  - payload: 4-byte length `M` (u32) = number of entries, then `M` times: (key: string encoded as 4-byte length + UTF-8 bytes) then a serialized `Value` for the property value
- `0x07` — Null
  - payload: none

(These tags match the VM's `read_value` implementation.)

## Top-level op encoding

Each op begins with a single opcode byte. The opcode values and payload formats used by the runtime (as implemented in `core/src/vm.rs`) are listed below.

Legend: `u8` = 1 byte, `u32` = 4-byte little-endian, `u64` = 8-byte little-endian. Register indices and label ordinals are encoded as `u32` in the stream.

-- Control & constants --

- `0x01` LConst
  - payload: `dest:u32` + `Value` (serialized)
  - Semantics: write value into register `dest`.

- `0x02` LLocal
  - payload: `dest:u32` `local:u32`
  - Semantics: load function-local `local` into register `dest` (frame lookup).

- `0x03` SLocal
  - payload: `src:u32` `local:u32`
  - Semantics: store register `src` into function-local index `local`.

-- Arithmetic ops (all have payload `dest:u32 a:u32 b:u32`) --

- `0x10` Add
- `0x11` Sub
- `0x12` Mul
- `0x13` Div
- `0x14` Mod

-- Comparisons (payload `dest:u32 a:u32 b:u32`) --

- `0x20` Eq
- `0x21` Neq
- `0x22` Lt
- `0x23` Lte
- `0x24` Gt
- `0x25` Gte

The VM uses numeric coercion when possible (integers and floats are compared numerically when both sides can be interpreted as numbers).

-- Logical ops --

- `0x26` And (payload `dest,a,b`)
- `0x27` Or  (payload `dest,a,b`)
- `0x28` Not (payload `dest,src`)

-- Inc/Dec --

- `0x30` Inc (payload `dest:u32`) — increments integer in register if it is Int
- `0x31` Dec (payload `dest:u32`) — decrements integer in register if it is Int

-- Labels & branches --

- `0x40` Label
  - payload: string (4-byte length + UTF-8 bytes)
  - The parser records the op index for the Label string and exposes it for `CallLabel` resolution.

- `0x41` Jump
  - payload: `target:u32` (op index)
  - Semantics: unconditional jump to op index `target`.

- `0x42` BrTrue
  - payload: `cond:u32 target:u32` — jump to `target` if register `cond` as-boolean is true

- `0x43` BrFalse
  - payload: `cond:u32 target:u32` — jump to `target` if register `cond` as-boolean is false

- `0x50` Halt
  - payload: none — stop execution

-- Calls --

- `0x70` Call
  - payload: `dest:u32 func:u32 argc:u32` then `argc` times `arg_reg:u32`
  - Semantics: evaluate function register `func`; the VM expects it to contain a `Symbol` naming a host function (e.g. `"say"`). The VM runs the host function and writes the return Value into `dest`.

- `0x71` CallLabel
  - payload: `dest:u32 label_index:u32 argc:u32` then `argc` times `arg_reg:u32`
  - Semantics: call a labeled function in the bytecode. The `label_index` corresponds to a label ordinal `L{n}` (the code uses label names like `L{n}` where `n` is ordinal). The VM resolves label -> op index, pushes a new frame seeded with the argument registers (copied into `frame.locals[0..]`), sets `return_pc` and `return_reg`, then jumps to the resolved label's op index + 1.

-- Arrays, objects, and members --

- `0x90` ArrayNew
  - payload: `dest:u32 len:u32` then `len` times `elem_reg:u32`
  - Semantics: build a new runtime `Array` by cloning values from the listed registers into a new Vector and store in `dest`.

- `0x91` ArrayGet
  - payload: `dest:u32 array:u32 index:u32`
  - Semantics: read `array` register (must be Array), read `index` register (must be Int), return item or Null.

- `0x92` ArraySet
  - payload: `array:u32 index:u32 src:u32`
  - Semantics: write into array (may create/resize array if destination isn't an array).

- `0x93` GetProp
  - payload: `dest:u32 obj:u32 key:u32`
  - Semantics: for `Object` or `Array` or `Str` handle `length` property and object properties. Otherwise return Null.

- `0x94` SetProp
  - payload: `obj:u32 key:u32 src:u32`
  - Semantics: mutate or create object at `obj` and set `key` -> value.

- `0x95` LoadGlobal
  - payload: `dest:u32 src:u32`
  - Semantics: copy register `src` (typically a module-level register) into `dest`. Used by finalized FunctionBuilder code to materialize module/global values into function-local registers.

-- Return --

- `0x80` Ret
  - payload: `src:u32` — pop frame; write `src` into caller's `return_reg` (if present) and jump to `return_pc`.

## Runtime `Value` shape (VM runtime)

At runtime the VM uses the following `Value` variants:

- `Int(i64)`, `Float(f64)`, `Bool(bool)`, `Str(String)`, `Symbol(String)`, `Array(Vec<Value>)`, `Object(HashMap<String, Value>)`, and `Null`.

The VM implements:

- `as_bool()` to coerce values for branching.
- `numeric_bin` and `numeric_cmp` helpers for arithmetic/comparison with numeric coercion when possible.

## Label resolution & CallLabel semantics

- Labels are emitted as string names (`Label` op). The parser records the op index for each label name.
- `CallLabel` uses a numeric `label_index` (the code uses a naming scheme like `L{n}` where `n` is an ordinal). The VM resolves the constructed `L{n}` to an op index via the label_by_name map and jumps there.

## Error cases & limits

- The VM returns errors for invalid header/magic/version, unknown opcode codes, and unresolved CallLabel labels.
- `Call` only supports `Symbol` values (host functions) in this prototype VM.
- There is a step limit enforced (200 steps) to prevent infinite loops during testing.

## Example: small bytecode sequence (pseudocode)

This example is conceptual and shows the ops rather than exact binary bytes. A minimal program:

1. LConst r0 <- Str("Hello")
2. LConst r1 <- Symbol("say")
3. Call dest=r2 func=r1 args=[r0]
4. Halt

Binary encoding (conceptual):

- Header: `MSBC` `u32 version=1` `u32 op_count=4`
- Op 1: `0x01` dest=0x00000000 value(tag=0x04 Str len=5 "Hello")
- Op 2: `0x01` dest=0x00000001 value(tag=0x05 Symbol len=3 "say")
- Op 3: `0x70` dest=0x00000002 func=0x00000001 argc=0x00000001 arg0=0x00000000
- Op 4: `0x50` (Halt)

(For a real byte array, serialize the fields as little-endian integers and UTF-8 bytes as described above.)

## Notes & rationale

- The MSBC format is intentionally small and easy to parse — it maps directly from the IR used during lowering and is convenient for a simple VM implementation.
- `LoadGlobal` exists to support the FunctionBuilder finalize step: it allows builder-local code to copy module-level registers into function locals in a way that survives register remapping.
- `Symbol` values are used to represent names for host functions and for situations where a genuine runtime object was not resolved at lowering time (though the lowering now prefers materializing real object registers when possible).

## Appendix: opcode summary table

- 0x01 LConst(dest:u32, Value)
- 0x02 LLocal(dest:u32, local:u32)
- 0x03 SLocal(src:u32, local:u32)
- 0x10 Add(dest,a,b)
- 0x11 Sub(dest,a,b)
- 0x12 Mul(dest,a,b)
- 0x13 Div(dest,a,b)
- 0x14 Mod(dest,a,b)
- 0x20 Eq(dest,a,b)
- 0x21 Neq(dest,a,b)
- 0x22 Lt(dest,a,b)
- 0x23 Lte(dest,a,b)
- 0x24 Gt(dest,a,b)
- 0x25 Gte(dest,a,b)
- 0x26 And(dest,a,b)
- 0x27 Or(dest,a,b)
- 0x28 Not(dest,src)
- 0x30 Inc(dest)
- 0x31 Dec(dest)
- 0x40 Label(string)
- 0x41 Jump(target:u32)
- 0x42 BrTrue(cond:u32,target:u32)
- 0x43 BrFalse(cond:u32,target:u32)
- 0x50 Halt
- 0x70 Call(dest, func, argc, args...)
- 0x71 CallLabel(dest, label_index, argc, args...)
- 0x80 Ret(src)
- 0x90 ArrayNew(dest,len,elems...)
- 0x91 ArrayGet(dest,array,index)
- 0x92 ArraySet(array,index,src)
- 0x93 GetProp(dest,obj,key)
- 0x94 SetProp(obj,key,src)
- 0x95 LoadGlobal(dest,src)

---

If you'd like, I can also:

- Add a small reference program and its exact serialized byte sequence as a hex dump.
- Add a `disasm` example that shows produced human-readable ops (or enhance `cli` disassembly output).

Tell me which follow-up you prefer and I'll implement it next.
