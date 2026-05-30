//! Parse-wide context for one invocation over input lifetime `'src`.
//!
//! # Invariants
//!
//! - **`memo_store`** — Parse-scoped; see [`crate::memo_store`] for the per-parser-id typing
//!   contract. [`crate::matcher::commit_matcher::CommitMatcher`] swaps memo stores during recovery
//!   so the inner branch explores with an empty cache without mutating the outer parse’s memos.
//! - **Error stacks** — [`crate::matcher::MatchRunner::run_match`] records a stack watermark on
//!   entry and truncates [`ParserContext::error_stack`] on matcher failure or `Err`, pairing
//!   [`crate::error::ErrorHandler::register_start`] / `register_success` / `register_failure`.
//!
//! Error-recovery pass selection is via [`crate::mode::Mode`] at the parse driver, not context fields.

use std::collections::HashSet;

use crate::cache::Cache;
use crate::error::ParserError;
#[cfg(feature = "parser-trace")]
use crate::trace::TraceState;

/// Holds memoization, collected errors, and recovery state for one parse.
///
/// The lifetime `'src` matches the input parse invocation (see [`crate::input::InputStream`]).
pub struct ParserContext<'src> {
    pub cache: Cache<'src>,
    pub error_sink: Vec<ParserError>,
    pub registered_error_set: HashSet<(usize, usize)>,
    pub error_stack: Vec<ParserError>,
    #[cfg(feature = "parser-trace")]
    pub(crate) trace: Option<TraceState>,
    _marker: std::marker::PhantomData<&'src ()>,
}

impl<'src> ParserContext<'src> {
    #[inline]
    pub fn new() -> Self {
        Self {
            cache: Cache::new(),
            error_sink: Vec::new(),
            registered_error_set: HashSet::new(),
            error_stack: Vec::new(),
            #[cfg(feature = "parser-trace")]
            trace: None,
            _marker: std::marker::PhantomData,
        }
    }

    #[inline]
    pub fn get_errors(mut self) -> Vec<ParserError> {
        // return combined errors from error_sink and error_stack
        self.error_sink.extend(self.error_stack);
        self.error_sink
    }

    #[inline]
    pub fn push_stack_error(&mut self, error: ParserError) {
        self.error_stack.push(error);
    }
}
