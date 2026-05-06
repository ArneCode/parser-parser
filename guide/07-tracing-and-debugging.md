# Tracing and Debugging

`marser` can emit structured parser runtime events when built with the
`parser-trace` feature.

## Quickstart

Run the JSON example with tracing enabled:

```bash
cargo run -p marser --features "parser-erased parser-trace" --example json -- tests/data/json1.json
```

Write a trace file from the same example:

```bash
cargo run -p marser --features "parser-erased parser-trace" --example json -- tests/data/json1.json --trace-file /tmp/json-trace.json
```

Open that trace in the TUI viewer:

```bash
cargo run -p marser-trace-viewer -- --trace /tmp/json-trace.json --source tests/data/json1.json
```

## Learn by example: `examples/json.rs`

If you want to get an immediate feel for what tracing looks like in a realistic
grammar, start with `examples/json.rs`.

Why this example is useful:

- it traces semantic branches (`object`, `array`, `string`, `number`, ...)
- it includes separators and selected recovery paths, so you can see failures
  and recovery decisions in replay
- it supports `--trace-file`, so you can produce traces without writing custom
  driver code

Suggested flow:

1. Open `examples/json.rs` and look for `.trace()` placements in the grammar.
2. Run:

```bash
cargo run -p marser --features "parser-erased parser-trace" --example json -- tests/data/json1.json --trace-file /tmp/json-trace.json
```

3. Replay:

```bash
cargo run -p marser-trace-viewer -- --trace /tmp/json-trace.json --source tests/data/json1.json
```

This gives you a concrete baseline for deciding where to place `.trace()`
markers in your own grammar.

## Which API should I use?

- `marser::parse(parser, src)`: normal parse, no trace collection.
- `marser::parse_with_trace(parser, src)`: parse + in-memory `TraceSession`.
- `marser::parse_with_trace_to_file(parser, src, path, format)`: parse + write
  trace directly to disk (good for larger traces and tooling workflows).

Tracing does not change parser behavior; it only records runtime events.

`TraceSession` (from `marser-trace-schema`) supports:

- `events()` to inspect structured events.
- `write_json(...)` for a versioned object `{ "trace_version", "nodes" }`.
- `write_jsonl(...)` for line-delimited events.

For human-readable dumps in Rust, import
[`TraceSessionExt`](crate::trace::TraceSessionExt) and call
`to_text_tree()` / `to_timeline()`.

## Trace formats

- `json`: one versioned trace document. Good default for replay and sharing.
- `jsonl`: one event per line. Useful for stream processing and external tooling.

The viewer accepts both formats and can auto-detect from content.

## Marker placement guidelines

Tracing is marker-first and driven by explicit `.trace()` calls in your grammar.
Stepping quality depends on marker placement.

Recommended:

- Place `.trace()` on meaningful grammar branches (for example: `object`,
  `array`, `string`, `number`).
- Trace separators and recovery points only when they help explain behavior.
- Keep markers sparse enough that each step answers "what parser decision
  happened next?".

Avoid:

- Tracing every tiny token when debugging high-level grammar flow.
- Placing many markers in whitespace-only paths unless whitespace handling is
  the bug you are investigating.

## Event kinds

Each explicit `.trace()` marker emits:

- marker `start` (`ParserEnter`)
- marker `end` with outcome:
  - `ParserExit` for success
  - `MatchFail` for soft fail
  - `MatchHardError` for hard error
- optional `marker_failure` snapshot on failed marker ends

Marker events share a `trace_marker_id`, which allows deterministic matching
between start/end events in replay tools.

## Rule source metadata

Explicit marker events can include optional rule identity metadata:

- `rule_id` (stable within one parse session)
- `rule_name` (derived from labels when available)
- `rule_file`, `rule_line`, `rule_column`

These fields support debugger-style UIs that map runtime events back to grammar
source locations.

## Replay stepping (viewer crate)

The `marser-trace-viewer` crate implements stepping and replay over a loaded
`TraceSession` (see `marser_trace_viewer::replay`). It depends on
`marser-trace-schema`, not on `marser`.

Run from workspace root:

```bash
cargo run -p marser-trace-viewer -- --trace path/to/trace.jsonl --source path/to/input.txt
```

Arguments:

- `--trace <path>` (required): trace file in JSON or JSONL
- `--source <path>` (optional): source file used for span preview
- `--format json|jsonl` (optional): force format, otherwise auto-detected

Key bindings:

- `i`: step into (next explicit trace start, including nested)
- `s`: step over (current trace start -> matching end -> next trace start)
- `u`: step up/out (parent trace end -> next start after parent end)
- `backspace`: return to previous displayed trace start
- `q`: quit

### Exact stepping contract

- **Visible step targets**: only explicit `.trace()` **start** events.
- **Startup position**: auto-advance to first visible user trace start
  (skipping bootstrap markers from `marser/src/lib.rs`).
- **`s` (StepOver)**:
  - from current trace start `S`, find matching end `E`
  - move to first visible trace start strictly after `E`
  - if none exists: parse complete
- **`i` (StepInto)**:
  - move to first visible trace start strictly after current start
  - nested starts are included naturally by execution order
- **`u` (StepOut)**:
  - find nearest parent trace span containing current start
  - jump to first visible trace start strictly after that parent end
  - if no parent/next exists: parse complete
- **`backspace`**:
  - return to the previous displayed trace start from linear history
  - no parent/sibling inference

## Troubleshooting

- No trace output: confirm `parser-trace` is enabled in your Cargo command.
- Slow parse with tracing: expected; tracing adds collection overhead.
- Hard-to-follow stepping: reduce marker density and prefer semantic trace points.

## Performance notes

Without `parser-trace`, instrumentation is compiled out. With `parser-trace`,
marker collection adds overhead, so prefer enabling it for debugging, testing,
and tooling rather than production hot paths.
