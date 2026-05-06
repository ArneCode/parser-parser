# marser-trace-schema

Serde types and I/O for **marser** parser traces (JSON envelope with `trace_version`, or JSONL lines of [`NodeTrace`](src/event.rs)).

- **Version policy**: see [`SUPPORTED_TRACE_VERSION_MIN`](src/version.rs) / `MAX` and `check_trace_version`.
- **Forward compatibility**: unknown `TraceEventKind` strings deserialize to `TraceEventKind::Unknown`.

This crate is intentionally small (no parser runtime, no UI). The `marser` crate
pulls it in as an optional dependency when the `parser-trace` feature is enabled.
