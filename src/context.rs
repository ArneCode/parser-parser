//! Parse-wide context for one invocation over input lifetime `'src`.
//!
//! # Invariants
//!
//! - **`is_in_error_recovery`** — Only the top-level driver sets this for the second pass after
//!   [`crate::error::MatcherRunError::RetryRerunNeeded`]. Nested code must not flip it. A fresh
//!   [`ParserContext::new`] is used between passes so memoization and recovery-local state do not
//!   leak across attempts.
//! - **`memo_store`** — Parse-scoped; see [`crate::memo_store`] for the per-parser-id typing
//!   contract. [`crate::matcher::commit_matcher::CommitMatcher`] swaps memo stores during recovery
//!   so the inner branch explores with an empty cache without mutating the outer parse’s memos.
//! - **Error stacks** — [`crate::matcher::MatchRunner::run_match`] records a stack watermark on
//!   entry and truncates [`ParserContext::error_stack`] on matcher failure or `Err`, pairing
//!   [`crate::error::ErrorHandler::register_start`] / `register_success` / `register_failure`.

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
    pub is_in_error_recovery: bool,
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
            is_in_error_recovery: false,
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
