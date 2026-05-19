//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.
//!
//! Types and serde I/O for marser parser trace logs (JSON / JSONL).
//!
//! **Experimental:** APIs and serialized trace formats may change between releases;
//! see the crate README on GitHub for the current stability note.

mod event;
mod io;
mod rule;
mod session;
mod version;

pub use event::{
    ExplicitMarkerEndOutcome, NodeTrace, NodeTraceKind, NodeTraceStatus, TraceEventKind,
    TraceMarkerFailureSnapshot, TraceMarkerPhase,
};
pub use io::{TraceFormat, detect_trace_format, load_json, load_jsonl, load_trace_file};
pub use rule::{RuleIdentity, RuleSourceMetadata, TraceLocation};
pub use session::TraceSession;
pub use version::{
    UnsupportedTraceVersion, SCHEMA_VERSION, SUPPORTED_TRACE_VERSION_MAX,
    SUPPORTED_TRACE_VERSION_MIN, check_trace_version,
};
