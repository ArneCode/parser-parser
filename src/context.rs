use std::any::Any;
use std::collections::{HashMap, HashSet};

use crate::error::ParserError;
#[cfg(feature = "parser-trace")]
use crate::trace::TraceState;

/// Parse-wide state shared by matchers and parsers.
///
/// `is_in_error_recovery` is set to `true` only by the top-level parse driver for the second
/// full-file pass after [`crate::error::MatcherRunError::RetryRerunNeeded`]. Nested combinators
/// must not flip it. Between passes the driver may call [`Self::reset_for_global_recovery_reparse`]
/// so memoization and transient diagnostics from the discarded attempt do not affect the recovery parse.
pub struct ParserContext {
    pub memo_table: HashMap<(usize, usize), Box<dyn Any>>,
    pub error_sink: Vec<ParserError>,
    pub registered_error_set: HashSet<(usize, usize)>,
    pub error_stack: Vec<ParserError>,
    pub is_in_error_recovery: bool,
    #[cfg(feature = "parser-trace")]
    pub(crate) trace: Option<TraceState>,
}

impl ParserContext {
    pub fn new() -> Self {
        Self {
            memo_table: HashMap::new(),
            error_sink: Vec::new(),
            registered_error_set: HashSet::new(),
            error_stack: Vec::new(),
            is_in_error_recovery: false,
            #[cfg(feature = "parser-trace")]
            trace: None,
        }
    }

    /// Clears memo and diagnostic accumulators before the global recovery re-parse.
    /// Only the top-level driver should call this between attempt 1 and 2.
    pub(crate) fn reset_for_global_recovery_reparse(&mut self) {
        self.memo_table.clear();
        self.error_sink.clear();
        self.registered_error_set.clear();
        self.error_stack.clear();
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
