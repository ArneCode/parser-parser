# Tracing and Debugging

`marser` can emit detailed parser runtime events when built with the
`parser-trace` feature.

## Enable tracing

Add the feature when checking, testing, or running examples:

```bash
cargo run --features "parser-erased parser-trace" --example json -- tests/data/json1.json
```

## API overview

Tracing does not change the default parse API:

- `marser::parse(parser, src)` stays unchanged.
- `marser::parse_with_trace(parser, src)` returns parse output, parser errors,
  and a `TraceSession`.

`TraceSession` supports:

- `events()` to inspect structured events.
- `write_json(...)` for JSON arrays.
- `write_jsonl(...)` for line-delimited events.
- `to_text_tree()` and `to_timeline()` for quick human-readable output.

## Event kinds

The trace stream includes:

- matcher lifecycle (`MatchEnter`, `MatchSuccess`, `MatchFail`, `MatchBacktrack`)
- hard failures (`MatchHardError`)
- choice behavior (`ChoiceStart`, arm start/success/fail, all-failed)
- commit behavior (`CommitPrefixMatched`, second-pass start/success/fail)
- error recovery behavior (`RecoverAttempt`, `RecoverSuccess`, `RecoverFail`)
- capture/parser boundaries (`CaptureEnter`, `CaptureExit`, `ParserEnter`, `ParserExit`)

This gives enough information to reconstruct parser control flow and backtracking.

For stepping, the viewer now prioritizes explicit `.trace(...)` markers:

- Each `.trace()` emits a marker `start` and `end` event pair.
- Marker events carry a shared `trace_marker_id`.
- Default stepping only navigates explicit marker starts.

## Rule source metadata

Events can also include optional rule identity metadata:

- `rule_id` (stable within one parse session)
- `rule_name` (derived from labels when available)
- `rule_file`, `rule_line`, `rule_column`

These fields are designed to support side-by-side debugger UIs that need to map
runtime steps back to grammar source code.

## Replay debugger foundation

`trace::debug_protocol` contains replay primitives that can step through a
recorded trace and stop on breakpoints:

- event-kind breakpoints
- label breakpoints
- position-range breakpoints

This layer is protocol-only so UIs can be built separately (TUI, web, editor).

## Replay TUI viewer

`marser` ships with a replay-only TUI trace viewer binary:

```bash
cargo run --features "parser-erased parser-trace" --bin trace_viewer -- --trace path/to/trace.jsonl --source path/to/input.txt
```

Arguments:

- `--trace <path>` (required): trace file in JSON or JSONL
- `--source <path>` (optional): source file used for span preview
- `--format json|jsonl` (optional): force parser format, otherwise auto-detected

Key bindings:

- `i`: step into (next explicit trace start, including nested)
- `s`: step over (current trace start -> matching end -> next trace start)
- `u`: step up/out (parent trace end -> next start after parent end)
- `backspace`: return to previous displayed trace start
- `t`: toggle hiding non-marker events
- `q`: quit

### Trace-only stepping model

To get deterministic stepping in grammar order, place `.trace()` only at meaningful grammar usage points.
By default, the viewer highlights and steps through those `.trace()` locations.

#### Exact viewer behavior contract

- **Visible step targets**: only explicit `.trace()` **start** events are considered step points.
- **Startup position**: viewer auto-advances to first visible user trace start (skipping bootstrap markers from `src/lib.rs`).
- **`s` (StepOver)**:
  - from current trace start `S`, find matching end `E`,
  - then move to first visible trace start strictly after `E`,
  - if none exists: parse complete.
- **`i` (StepInto)**:
  - move to first visible trace start strictly after current start,
  - nested starts are included naturally by execution order.
- **`u` (StepOut)**:
  - find nearest parent trace span that contains current start,
  - jump to first visible trace start strictly after that parent end,
  - if no parent/next exists: parse complete.
- **`backspace`**:
  - returns to the immediately previous **displayed** trace start from linear display history,
  - does not infer parent/sibling logic and does not toggle between two nodes unless that was truly the last two displayed.

## Performance notes

Without `parser-trace`, instrumentation is compiled out.
With `parser-trace`, marker and runtime event collection adds overhead, so prefer enabling it
for debugging and tests rather than production parsing hot paths.

