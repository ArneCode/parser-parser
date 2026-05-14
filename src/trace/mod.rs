//! Tracing helpers and (with the `parser-trace` feature) re-exports from
//! [`marser_trace_schema`](https://docs.rs/marser-trace-schema).
//!
//! **Experimental:** When `parser-trace` is enabled, trace formats and supporting
//! crates may evolve; see the project README and [`crate::guide::tracing_and_debugging`].
//!
//! [`Traced`] / [`WithTrace`] wrap parsers with explicit `.trace()` markers without
//! changing parse results.

#[cfg(feature = "parser-trace")]
pub use marser_trace_schema::*;

#[cfg(feature = "parser-trace")]
mod context_trace_impl;

#[cfg(feature = "parser-trace")]
pub(crate) use context_trace_impl::TraceState;

mod traced;

pub use traced::{Traced, WithTrace};

#[cfg(feature = "parser-trace")]
impl From<&crate::error::FurthestFailError> for TraceMarkerFailureSnapshot {
    #[inline]
    fn from(e: &crate::error::FurthestFailError) -> Self {
        Self {
            span_start: e.span.0,
            span_end: e.span.1,
            expected: e.expected.clone(),
            summary: e.to_string(),
        }
    }
}

#[cfg(feature = "parser-trace")]
impl From<&crate::error::MatcherRunError> for TraceMarkerFailureSnapshot {
    #[inline]
    fn from(e: &crate::error::MatcherRunError) -> Self {
        match e {
            crate::error::MatcherRunError::FurthestFail(f) => Self::from(f),
            crate::error::MatcherRunError::RetryRerunNeeded => Self {
                span_start: 0,
                span_end: 0,
                expected: vec![],
                summary: "retry rerun needed".to_string(),
            },
        }
    }
}
