# Integration tests (`tests/`)

## Run the full test suite

From the **repository root**, this runs every `marser` integration test in `tests/`, including the ones that are normally gated behind Cargo features:

```bash
git submodule update --init tests/JSONTestSuite
cargo test -p marser --features "parser-trace json-testsuite"
```

- **`parser-trace`** — compiles and runs `trace_harness.rs`.
- **`json-testsuite`** — compiles and runs `json_testsuite.rs` against [JSONTestSuite](https://github.com/nst/JSONTestSuite) (the `git submodule` line is required so `tests/JSONTestSuite/test_parsing/` exists).

Then run tests for the other workspace members (their crates live next to their own `Cargo.toml` files, not under this folder):

```bash
cargo test -p marser-trace-schema
cargo test -p marser-trace-viewer
```

The `marser_macros` package currently has no `#[test]` targets; `cargo test -p marser_macros` only checks that the proc-macro crate builds.

---

Each `tests/<name>.rs` file is a separate integration-test binary (filter with `cargo test -p marser --test <name>`). They exercise the published `marser` API against grammars and fixtures under this directory.

For **day-to-day** runs, `cargo test -p marser` is enough (see the repo `README.md`).

## Quick commands (typical / partial)

All default integration tests that do **not** need extra features:

```bash
cargo test -p marser
```

(From the repo root you can omit `-p marser` when the selected package is already `marser`.)

One integration test crate:

```bash
cargo test -p marser --test json_harness
```

Compile-fail UI tests (`trybuild`):

```bash
cargo test -p marser --test capture_ui
```

## Test crates (root `marser` package)

| File | Purpose |
|------|---------|
| `capture_ui.rs` | [`trybuild`](https://docs.rs/trybuild): `capture!` / `bind!` pass + compile-fail cases under `tests/ui/` with golden `.stderr`. |
| `json_harness.rs` | [`examples/json/grammar.rs`](../examples/json/grammar.rs) on small fixtures in `tests/data/json*.json` (valid = no recovery diagnostics; invalid = recovered AST + diagnostics). |
| `json_suite.rs` | Smoke tests for `TokenParser` + `parse`. |
| `json_testsuite.rs` | [JSONTestSuite](https://github.com/nst/JSONTestSuite) corpus (`tests/JSONTestSuite/test_parsing/`). **Requires** `json-testsuite`. See below. |
| `memoized_borrow.rs` | Regression: `Memoized` + `capture!` with outputs that borrow the input. |
| `mini_language_harness.rs` | `examples/mini_language` on `.ml` fixtures under `tests/data/mini_language/` (parse + diagnostics + non-interactive run). |
| `repeated_bind_capture.rs` | Regression: repeated compatible `*name` binds in one `capture!`. |
| `trace_harness.rs` | Parser tracing (`parse_with_trace`, markers, file output). **Requires** `parser-trace`. |

### `capture_ui` / `trybuild`

Regenerate golden compiler output after intentional diagnostic changes or a new Rust toolchain:

```bash
TRYBUILD=overwrite cargo test -p marser --test capture_ui
```

Review diffs, then re-run without `TRYBUILD=overwrite` until green.

### JSONTestSuite (`json_testsuite`)

1. Initialize the submodule (once per clone):

   ```bash
   git submodule update --init tests/JSONTestSuite
   ```

2. Run the harness:

   ```bash
   cargo test -p marser --features json-testsuite --test json_testsuite
   ```

Behavior is documented in the module-level comments in `json_testsuite.rs` (prefixes `y_` / `n_` / `i_`, UTF-8-only inputs, skipped pathological structure files).

Optional **per-file** helpers (each subprocess sets `JSONSUITE_FILE` and invokes `nst_single_file_from_env`):

```bash
python3 tests/run_jsonsuite_single.py tests/JSONTestSuite/test_parsing/y_object.json --mode release
python3 tests/run_jsonsuite_matrix.py --mode both
python3 tests/find_min_stack.py tests/JSONTestSuite/test_parsing/<file>.json --mode release
```

For very deep inputs you may need a larger stack, e.g. `RUST_MIN_STACK=…` (see `find_min_stack.py`).

### Trace harness

```bash
cargo test -p marser --features parser-trace --test trace_harness
```

## Fixtures and data

- `tests/data/` — small JSON and mini-language (`.ml`) samples used by `json_harness` and `mini_language_harness`.
- `tests/JSONTestSuite/` — third-party corpus (submodule); parsing vectors live under `test_parsing/`.
- `tests/ui/` — sources + `.stderr` for `capture_ui` `trybuild` cases.

## Benchmarks (`benches/`)

Criterion bench **`json_parse`** (same grammar as `json_harness` via `examples/json/grammar.rs`):

```bash
cargo bench --bench json_parse
```

See the repo `README.md` for Criterion HTML output, optional **gnuplot**, and **flamegraph** / `perf` notes (WSL2 often needs a generic `perf` or `PERF=…`).

## Workspace crates

See **Run the full test suite** above for `cargo test -p marser-trace-schema` and `cargo test -p marser-trace-viewer`. Test sources live next to each crate’s `Cargo.toml`.
