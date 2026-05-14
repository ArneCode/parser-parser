//! Match attempt bookkeeping for furthest-fail and recovery diagnostics.
//!
//! # Contract
//!
//! [`MatchRunner`](crate::matcher::MatchRunner) calls [`ErrorHandler::register_start`] before each
//! `match_with_runner` attempt, then exactly one of [`ErrorHandler::register_success`] or
//! [`ErrorHandler::register_failure`] with the returned indexer. [`register_watermark`] tracks the
//! furthest input position seen during the attempt for diagnostics.

use std::{collections::HashSet, fmt::Display};

pub(crate) enum ErrorHandlerChoice<'a> {
    Empty(&'a mut EmptyErrorHandler),
    Multi(&'a mut MultiErrorHandler),
}

pub(crate) trait ErrorHandler {
    type Indexer;
    const IS_REAL: bool;

    fn new(start_pos: usize) -> Self
    where
        Self: Sized;

    fn register_start(&mut self, pos: usize) -> Self::Indexer;
    fn register_failure<L: Display + 'static>(&mut self, label: Option<L>, idx: Self::Indexer);
    fn register_success(&mut self, idx: Self::Indexer);
    fn register_watermark(&mut self, pos: usize);
    fn to_choice(&mut self) -> ErrorHandlerChoice<'_>;
    #[inline]
    fn is_real(&self) -> bool {
        Self::IS_REAL
    }
}
#[derive(Default)]
pub(crate) struct EmptyErrorHandler;

impl ErrorHandler for EmptyErrorHandler {
    type Indexer = ();
    const IS_REAL: bool = false;

    #[inline(always)]
    fn new(_start_pos: usize) -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    #[inline(always)]
    fn register_start(&mut self, _pos: usize) -> Self::Indexer {}
    #[inline(always)]
    fn register_failure<L: Display>(&mut self, _label: Option<L>, _idx: Self::Indexer) {}
    #[inline(always)]
    fn register_success(&mut self, _idx: Self::Indexer) {}
    #[inline(always)]
    fn register_watermark(&mut self, _pos: usize) {}
    #[inline(always)]
    fn to_choice(&mut self) -> ErrorHandlerChoice<'_> {
        ErrorHandlerChoice::Empty(self)
    }
}

pub(crate) struct MultiErrorHandler {
    best_failure_slice: (usize, usize),
    slice_stack: Vec<(usize, usize)>,
    expected_labels: Vec<(String, usize)>,
    time: usize,
}

impl MultiErrorHandler {}

pub(crate) struct MultiErrorHandlerIndex {
    slice_idx: usize,
    creation_time: usize,
}
impl MultiErrorHandler {
    #[inline]
    fn pop_slice_stack(&mut self, idx: &MultiErrorHandlerIndex) {
        if self.slice_stack.len() != idx.slice_idx + 1 {
            panic!(
                "Mismatched register_success: expected slice_idx {}, but got {}",
                self.slice_stack.len() - 1,
                idx.slice_idx
            );
        }
        if let Some((_start, end)) = self.slice_stack.pop()
            && let Some(slice) = self.slice_stack.last_mut()
        {
            slice.1 = slice.1.max(end);
        }
    }
}
impl ErrorHandler for MultiErrorHandler {
    type Indexer = MultiErrorHandlerIndex;
    const IS_REAL: bool = true;

    #[inline]
    fn new(start_pos: usize) -> Self {
        Self {
            best_failure_slice: (0, 0),
            slice_stack: vec![(start_pos, start_pos)],
            expected_labels: Vec::new(),
            time: 0,
        }
    }

    #[inline]
    fn register_start(&mut self, pos: usize) -> Self::Indexer {
        self.slice_stack.push((pos, pos));
        self.time += 1;
        MultiErrorHandlerIndex {
            slice_idx: self.slice_stack.len() - 1,
            creation_time: self.time,
        }
    }

    #[inline]
    fn register_watermark(&mut self, pos: usize) {
        if let Some(slice) = self.slice_stack.last_mut() {
            slice.1 = slice.1.max(pos);
        }
    }

    #[inline]
    fn register_success(&mut self, idx: Self::Indexer) {
        self.pop_slice_stack(&idx);
    }

    fn register_failure<L: Display + 'static>(&mut self, label: Option<L>, idx: Self::Indexer) {
        let failure_slice = self.slice_stack[idx.slice_idx];
        self.pop_slice_stack(&idx);
        if label.is_none() {
            return;
        }

        // check if this error is less interesting than the best failure so far
        if failure_slice.1 < self.best_failure_slice.1
            || failure_slice.0 < self.best_failure_slice.0
        {
            // This error is worse, so ignore it.
            return;
        }
        // check if this error is more interesting than the best failure so far
        if failure_slice.1 > self.best_failure_slice.1
            || failure_slice.0 > self.best_failure_slice.0
        {
            // This error is better, so clear the previous errors and update the best failure slice.
            self.expected_labels.clear();
            self.best_failure_slice = failure_slice;
        }

        // remove all labels that were created after this label, meaning that they are shadowed by this label
        while let Some((_label, creation_time)) = self.expected_labels.last() {
            if *creation_time > idx.creation_time {
                self.expected_labels.pop();
            } else {
                break;
            }
        }
        // Now this error is at least as interesting as the best failure, so register it.
        self.expected_labels.push((label.unwrap().to_string(), idx.creation_time));
    }
    #[inline]
    fn to_choice(&mut self) -> ErrorHandlerChoice<'_> {
        ErrorHandlerChoice::Multi(self)
    }
}

// ── MultiErrorHandler → ParserError ─────────────────────────────────────────

impl MultiErrorHandler {
    /// Convert the accumulated error state into a [`FurthestFailError`].
    ///
    /// * If any errors were recorded, `span` covers `best_failure_slice`.
    /// * If no errors were recorded (e.g. the parse succeeded or never
    ///   encountered a labelled failure), falls back to the **bottom slice**
    ///   — the outermost span tracking how far the parser actually reached.
    pub fn to_parser_error(&self) -> crate::error::FurthestFailError {
        let has_errors = !self.expected_labels.is_empty();

        let span = if has_errors {
            self.best_failure_slice
        } else {
            // Bottom of the slice stack: the root span seeded by `new()`.
            // Its `.1` is the furthest watermark reached by the whole parse.
            self.slice_stack.first().copied().unwrap_or((0, 0))
        };

        // Deduplicate labels while preserving a deterministic order.
        let expected: Vec<String> = self
            .expected_labels
            .iter()
            .map(|(label, _)| format!("'{}'", label))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        crate::error::FurthestFailError {
            span,
            expected,
            annotations: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
        }
    }
}
