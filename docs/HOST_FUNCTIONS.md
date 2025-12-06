## Host functions (built-ins)

This document describes the built-in host functions the VM exposes to scripts
via `Value::Symbol` calls (e.g. `say`, `read`, `write`, `fmt`, `ask`). These
are implemented in `core/src/vm/host.rs`.

read (file reading and glob expansion)
- Purpose: read file contents for use inside scripts and stages.
- Signature: `read(pathOrPattern)` or `read(arrayOfPathsOrPatterns)`
- Behavior:
  - Accepts a single string argument or an array of strings.
  - Each string may be:
    - A literal path to a file (e.g. `"foo.txt"`) — the file is read and its
      contents included in the result.
    - A glob pattern (e.g. `"*.ms"`, `"src/*.cpp"`) — the pattern is expanded
      using the `glob` crate; all matching files are read and their contents
      included in the result.
  - Returns: an `Array` of file contents (each element is a `Str`) — the result
    is always an array (possibly empty). This makes idioms like `val = read("*"); say(val[0]);` safe.
  - Resolution: glob patterns are expanded relative to the current working
    directory used by the VM (the engine sets the working directory to the
    script's directory when running the script in-process), so patterns like
    `"./src/*.cpp"` will match files under the script folder.
  - Errors: a `glob error` is returned when the pattern cannot be parsed; a
    `read error for <path>` is returned when a concrete file fails to be
    read. A glob that matches zero files simply results in no items for that
    pattern (no error) — callers receive an empty array element for that
    pattern.

say
- Purpose: print values to stdout from scripts.
- Signature: `say(value)` — prints strings, arrays (each element on its own
  line if possible), symbols and debug-prints complex values.

fmt
- A small string formatter used from scripts. Signature: `fmt(formatString, args...)`.

ask
- Prompt the user for input on stdin. Attempts to parse typed input as `bool`,
  `int`, or `float` before returning a `Str` fallback.

write
- Write content to disk. Signature: `write(path, content)` — returns `Bool(true)`
  on success or an error string on failure.

Notes & guidance
- The VM always returns canonical runtime `Value` variants: `Array`, `Str`,
  `Int`, `Float`, `Bool`, `Symbol`, `Object` and `Null`.
- For `read`, if you need file *paths* rather than file contents, implement a
  small helper in script code that captures the path list or consider adding a
  host helper `read_paths` if callers commonly need paths.

Location in source
- Implementation: `core/src/vm/host.rs`

Example usages
- Read all .ms files under the script folder and print first file's content:

```
sources = read("*.ms");
say(sources[0]);
```

- Read specific files and patterns together:

```
files = read(["README.md", "examples/*.ms"]);
for f in files { say(f); }
```

If you'd like `read` to return objects with both `path` and `content`, or a
variant that returns paths only, tell me which format you prefer and I can
add it and update the examples accordingly.
