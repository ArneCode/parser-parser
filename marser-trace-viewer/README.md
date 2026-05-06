# marser-trace-viewer

Terminal UI for inspecting parser traces (JSON / JSONL). This crate depends on
`marser-trace-schema` and does not depend on `marser`.

## Run

From this workspace:

```bash
cargo run -p marser-trace-viewer -- --trace path/to/trace.json --source path/to/input.txt
```

Install as a binary:

```bash
cargo install marser-trace-viewer
marser-trace-viewer --trace path/to/trace.json --source path/to/input.txt
```

## Arguments

- `--trace <path>` (required): path to JSON or JSONL trace file.
- `--source <path>` (optional): source input used for span preview.
- `--format json|jsonl` (optional): force parser format. If omitted, format is
  auto-detected.

## Key bindings

- `i`: step into (next explicit trace start, including nested markers)
- `s`: step over (current start -> matching end -> next start)
- `u`: step out (parent end -> next start after parent end)
- `backspace`: jump back to previously displayed start
- `q`: quit

## Stepping model

The viewer intentionally prioritizes explicit `.trace()` markers:

- visible step targets are only marker **start** events
- each marker has a start/end pair linked by `trace_marker_id`
- startup jumps to the first visible user marker start

This keeps replay deterministic and aligned with grammar-level trace points.

## End-to-end example with `marser`

Generate a trace from the JSON example:

```bash
cargo run -p marser --features "parser-erased parser-trace" --example json -- tests/data/json1.json --trace-file /tmp/json-trace.json
```

Open it in the viewer:

```bash
cargo run -p marser-trace-viewer -- --trace /tmp/json-trace.json --source tests/data/json1.json
```

## Related docs

See `guide/07-tracing-and-debugging.md` for the tracing API (`parse_with_trace`,
`parse_with_trace_to_file`) and detailed replay semantics.
