# JSON example

**AI assistance:** This document was drafted with AI assistance. The maintainer reviewed it. If anything looks wrong, please [open an issue](https://github.com/ArneCode/marser/issues/new).

A full JSON parser that demonstrates **error recovery**, **inline and furthest-fail diagnostics**, and a recoverable AST with `JsonValue::Invalid` nodes. The CLI prints annotate-snippets diagnostics and a pretty-printed “recovered JSON” tree.

## Layout

| File | Role |
| ---- | ---- |
| [`main.rs`](main.rs) | CLI: read file, run grammar, print errors and recovered output |
| [`grammar.rs`](grammar.rs) | Grammar (`capture!`, `commit_on`, `recover_with`, labels, optional `WithTrace`) |

Cargo runs this as the **`json`** example (`examples/json/main.rs` is the example root).

## Run

From the repository root:

```bash
cargo run --example json --features annotate-snippets -- tests/data/json1.json
```

Other fixtures under [`tests/data/`](../../tests/data/):

- `json0.json` — valid sample
- `json1.json` — invalid (comma/bracket issues); matches the screenshot in the [repository README](../../README.md#error-output-sample)
- `json2.json`, `json3.json` — additional cases

## Optional: parser tracing

With **`parser-trace`** enabled, the same binary can record a trace file for the [trace viewer](../../marser-trace-viewer/README.md):

```bash
cargo run --example json --features "annotate-snippets parser-trace" -- \
  tests/data/json1.json --trace-file /tmp/json-trace.json

cargo run -p marser-trace-viewer -- --trace /tmp/json-trace.json --source tests/data/json1.json
```

Without `--trace-file`, tracing still runs in memory (no file written).

## Compared to the guide

The [worked JSON guide chapter](https://docs.rs/marser/latest/marser/guide/worked_json_example/index.html) builds a smaller grammar step by step. This example is the **production-style** version: richer numbers and strings, more recovery paths, and patterns used in benchmarks and doc scraping.

## Highlights in `grammar.rs`

- Borrowed string payloads and `Invalid(&str)` for skipped regions
- `commit_on` / `recover_with` for objects, arrays, and values
- `unwanted`, `if_error`, and labeled rules for clearer diagnostics
- `.erase_types()` on heavy `one_of` branches to keep compile times manageable
