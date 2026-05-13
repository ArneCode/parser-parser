use std::collections::HashSet;

use crate::error::ParserError;
use crate::memo_store::MemoStore;
#[cfg(feature = "parser-trace")]
use crate::trace::TraceState;

/// Parse-wide state shared by matchers and parsers.
///
/// `is_in_error_recovery` is set to `true` only by the top-level parse driver for the second
/// full-file pass after [`crate::error::MatcherRunError::RetryRerunNeeded`]. Nested combinators
/// must not flip it. Between passes the driver uses a fresh [`ParserContext::new`] so memoization
/// and transient diagnostics from the discarded attempt do not affect the recovery parse.
///
/// The lifetime `'src` matches the input parse invocation (see [`crate::input::InputStream`]).
pub struct ParserContext<'src> {
    pub memo_store: MemoStore<'src>,
    pub error_sink: Vec<ParserError>,
    pub registered_error_set: HashSet<(usize, usize)>,
    pub error_stack: Vec<ParserError>,
    pub is_in_error_recovery: bool,
    #[cfg(feature = "parser-trace")]
    pub(crate) trace: Option<TraceState>,
    _marker: std::marker::PhantomData<&'src ()>,
}

impl<'src> ParserContext<'src> {
    pub fn new() -> Self {
        Self {
            memo_store: MemoStore::default(),
            error_sink: Vec::new(),
            registered_error_set: HashSet::new(),
            error_stack: Vec::new(),
            is_in_error_recovery: false,
            #[cfg(feature = "parser-trace")]
            trace: None,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn get_errors(mut self) -> Vec<ParserError> {
        // return combined errors from error_sink and error_stack
        self.error_sink.extend(self.error_stack);
        self.error_sink
    }

    pub fn push_stack_error(&mut self, error: ParserError) {
        self.error_stack.push(error);
    }
}
