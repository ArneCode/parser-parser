//! Building blocks for [`Capture`], the parser that runs a [`crate::matcher::Matcher`] and
//! maps captured slots into a Rust value.
//!
//! Typical usage is through the `capture!` macro from `marser_macros`, which expands to
//! [`Capture::new`] and rewrites embedded `bind!(…)` / `bind_span!(…)` (and `*` / `?` bind
//! forms) into [`bind_result`], [`bind_span`], and [`Property`] helpers. You can also
//! construct [`Capture`] and matchers by hand using these types.

mod bound;
mod capture_parser;
mod match_result;
mod property;
mod result_binder;
mod slice_binder;
mod span_binder;

pub use bound::{BoundResult, BoundValue};
pub use capture_parser::Capture;
pub use property::{BindDebugInfo, MultipleProperty, OptionalProperty, Property, SingleProperty};
pub use result_binder::{ResultBinder, bind_result};
pub use slice_binder::{SliceBinder, bind_slice};
pub use span_binder::{SpanBinder, bind_span};

pub(crate) use match_result::MatchResult;
