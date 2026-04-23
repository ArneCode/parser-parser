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
    fn is_real(&self) -> bool {
        Self::IS_REAL
    }
}
#[derive(Default)]
pub(crate) struct EmptyErrorHandler;

impl ErrorHandler for EmptyErrorHandler {
    type Indexer = ();
    const IS_REAL: bool = false;

    fn new(_start_pos: usize) -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn register_start(&mut self, _pos: usize) -> Self::Indexer {}
    fn register_failure<L: Display>(&mut self, _label: Option<L>, _idx: Self::Indexer) {}
    fn register_success(&mut self, _idx: Self::Indexer) {}
    fn register_watermark(&mut self, _pos: usize) {}
    fn to_choice(&mut self) -> ErrorHandlerChoice<'_> {
        ErrorHandlerChoice::Empty(self)
    }
}

pub(crate) struct MultiErrorHandler {
    best_failure_slice: (usize, usize),
    slice_stack: Vec<(usize, usize)>,
    expected_labels: Vec<String>,
}

impl MultiErrorHandler {}

pub(crate) struct MultiErrorHandlerIndex {
    slice_idx: usize,
}
impl MultiErrorHandler {
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

    fn new(start_pos: usize) -> Self {
        Self {
            best_failure_slice: (0, 0),
            slice_stack: vec![(start_pos, start_pos)],
            expected_labels: Vec::new(),
        }
    }

    fn register_start(&mut self, pos: usize) -> Self::Indexer {
        self.slice_stack.push((pos, pos));
        MultiErrorHandlerIndex {
            slice_idx: self.slice_stack.len() - 1,
        }
    }

    fn register_watermark(&mut self, pos: usize) {
        if let Some(slice) = self.slice_stack.last_mut() {
            slice.1 = slice.1.max(pos);
        }
    }


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

        // Now this error is at least as interesting as the best failure, so register it.
        self.expected_labels.push(label.unwrap().to_string());
    }
    fn to_choice(&mut self) -> ErrorHandlerChoice<'_> {
        ErrorHandlerChoice::Multi(self)
    }
}

// ── MultiErrorHandler → ParserError ─────────────────────────────────────────

impl MultiErrorHandler {
    /// Convert the accumulated error state into a [`ParserError`].
    ///
    /// * If any errors were recorded, `span` covers `best_failure_slice`.
    /// * If no errors were recorded (e.g. the parse succeeded or never
    ///   encountered a labelled failure), falls back to the **bottom slice**
    ///   — the outermost span tracking how far the parser actually reached.
    pub fn to_parser_error(&self) -> FurthestFailError {
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
            .map(|e| format!("'{}'", e))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        FurthestFailError {
            span,
            expected,
            annotations: ParserErrorAnnotations::default(),
        }
    }
}

impl Display for FurthestFailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let expected_msg = match self.expected.len() {
            0 => "unexpected token".to_string(),
            1 => format!("expected {}", self.expected[0]),
            _ => {
                // Sorting for consistent output
                let mut sorted = self.expected.clone();
                sorted.sort();
                format!("expected one of {}", sorted.join(", "))
            }
        };

        // Output format: "expected one of String, Number at 10..15"
        write!(f, "{} at {}..{}", expected_msg, self.span.0, self.span.1)
    }
}

use std::fmt::{Debug, Formatter};

use crate::context::ParserContext;
use crate::error::{ExtraLabel, FurthestFailError, ParserError, ParserErrorAnnotations};

impl Debug for ExtraLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtraLabel")
            .field("span", &self.span)
            .field("message", &self.message)
            .field("color", &format_args!("{:?}", self.color))
            .finish()
    }
}

impl Debug for ParserErrorAnnotations {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserErrorAnnotations")
            .field("notes", &self.notes)
            .field("help", &self.help)
            .field("extra_labels", &self.extra_labels)
            .finish()
    }
}

impl Debug for FurthestFailError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserError")
            .field("span", &self.span)
            .field("expected", &self.expected)
            .field("annotations", &self.annotations)
            .finish()
    }
}
