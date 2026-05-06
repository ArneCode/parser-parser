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
    fn from(e: &crate::error::FurthestFailError) -> Self {
        Self {
            span_start: e.span.0,
            span_end: e.span.1,
            expected: e.expected.clone(),
            summary: e.to_string(),
        }
    }
}
