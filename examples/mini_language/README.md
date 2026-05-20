# Mini language example

**AI assistance:** This document was drafted with AI assistance. The maintainer reviewed it. If anything looks wrong, please [open an issue](https://github.com/ArneCode/marser/issues/new).

A small imperative language with **functions**, **operator precedence**, and a tiny **evaluator**. The binary parses a `.ml` file, prints diagnostics on failure or recovery, and evaluates successful parses to a value.

## Layout

| File | Role |
| ---- | ---- |
| [`../mini_language.rs`](../mini_language.rs) | Example entrypoint (`main`) |
| [`mod.rs`](mod.rs) | `parse_source`, `eval_parsed`, shared API for tests |
| [`grammar.rs`](grammar.rs) | Parser: statements, expressions, `FunctionDef` AST |
| [`eval.rs`](eval.rs) | Interpreter over the parsed AST |

Cargo runs this as the **`mini_language`** example; the `#[path = "mini_language/mod.rs"]` attribute in the entrypoint wires this directory into the example crate.

## Run

From the repository root:

```bash
# Valid program (prints evaluated result)
cargo run --example mini_language --features annotate-snippets -- \
  tests/data/mini_language/valid/fibonacci.ml

# Valid: control flow and calls
cargo run --example mini_language --features annotate-snippets -- \
  tests/data/mini_language/valid/call_and_return.ml

# Invalid: syntax recovery (diagnostics + recovered AST, exit code 1)
cargo run --example mini_language --features annotate-snippets -- \
  tests/data/mini_language/invalid/unexpected_semicolons.ml
```

Usage: `mini_language <path-to-script.ml>` (see `usage` in [`mini_language.rs`](../mini_language.rs)).

## Fixtures

Under [`tests/data/mini_language/`](../../tests/data/mini_language/):

| Path | Purpose |
| ---- | ------- |
| `valid/*.ml` | Programs that parse and evaluate |
| `invalid/*.ml` | Broken syntax or type errors; shows `ParserError::eprint_many` and recovered AST |

Integration tests in [`tests/mini_language_harness.rs`](../../tests/mini_language_harness.rs) exercise the same grammar and evaluator.

## Behavior

1. **Parse** — `get_mini_language_grammar().parse_str(source)` → `Ok((functions, errors))` or `Err(FurthestFailError)`.
2. **Recover** — If `errors` is non-empty, the CLI prints diagnostics and the recovered AST, then exits with code `1`.
3. **Evaluate** — If there are no parse errors, `eval_parsed` runs the functions and prints the resulting [`Value`](eval.rs).

Hard parse failures use `FurthestFailError::eprint` and exit `1`.

## Highlights in `grammar.rs`

- Precedence climbing / layered expression rules
- `commit_on` and recovery for statements and blocks
- `bind_slice!` and borrowed identifiers in the AST
- Patterns similar to the JSON example for local, committed errors
