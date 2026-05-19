> **AI assistance:** This document was written with AI assistance. The maintainer reviewed it and did not find errors.

# marser-trace-schema

**Experimental:** This crate’s public API and the JSON / JSONL trace formats may
change between releases without the same stability guarantees as the core
`marser` library. Pin versions and read release notes when upgrading.

Serde types and I/O for **marser** parser traces (JSON envelope with `trace_version`, or JSONL lines of [`NodeTrace`](src/event.rs)).

- **Version policy**: see [`SUPPORTED_TRACE_VERSION_MIN`](src/version.rs) / `MAX` and `check_trace_version`.
- **Forward compatibility**: unknown `TraceEventKind` strings deserialize to `TraceEventKind::Unknown`.

This crate is intentionally small (no parser runtime, no UI). The `marser` crate
pulls it in as an optional dependency when the `parser-trace` feature is enabled.
