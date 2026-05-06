#[cfg(feature = "parser-trace")]
pub use marser_trace_schema::*;

#[cfg(feature = "parser-trace")]
pub mod render_text;

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

/// Human-readable trace dumps (implemented for [`TraceSession`] from `marser-trace-schema`).
#[cfg(feature = "parser-trace")]
pub trait TraceSessionExt {
    fn to_text_tree(&self) -> String;
    fn to_timeline(&self) -> String;
}

#[cfg(feature = "parser-trace")]
impl TraceSessionExt for TraceSession {
    fn to_text_tree(&self) -> String {
        render_text::render_tree(self.events())
    }

    fn to_timeline(&self) -> String {
        render_text::render_timeline(self.events())
    }
}
