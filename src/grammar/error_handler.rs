use std::{collections::HashSet, fmt::Display};

use ariadne::{Color, Label, Report, ReportKind, Source};

pub(crate) enum ErrorHandlerChoice<'a> {
    Empty(&'a mut EmptyErrorHandler),
    Multi(&'a mut MultiErrorHandler),
}

pub trait ErrorHandler {
    type Indexer;

    fn new(start_pos: usize) -> Self
    where
        Self: Sized;

    fn register_start(&mut self, pos: usize) -> Self::Indexer;
    fn register_error<L: Display + 'static>(
        &mut self,
        label: L,
        idx: Self::Indexer,
        match_start: usize,
    );
    fn register_success(&mut self, idx: Self::Indexer);
    fn register_watermark(&mut self, pos: usize);
    fn to_choice(&mut self) -> ErrorHandlerChoice<'_>;
}
#[derive(Default)]
pub struct EmptyErrorHandler;

impl ErrorHandler for EmptyErrorHandler {
    type Indexer = ();

    fn new(_start_pos: usize) -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn register_start(&mut self, _pos: usize) -> Self::Indexer {}
    fn register_error<L: Display>(&mut self, _label: L, _idx: Self::Indexer, _match_start: usize) {}
    fn register_success(&mut self, _idx: Self::Indexer) {}
    fn register_watermark(&mut self, _pos: usize) {}
    fn to_choice(&mut self) -> ErrorHandlerChoice<'_> {
        ErrorHandlerChoice::Empty(self)
    }
}

pub struct MultiErrorHandler {
    best_failure_slice: (usize, usize),
    slice_stack: Vec<(usize, usize)>,
    errors: Vec<Option<Box<dyn Display>>>,
    errors_at_match_start: Vec<usize>, // indices of errors that occurred at their match_start
}

// impl Default for MultiErrorHandler {
//     fn default() -> Self {
//         Self::new()
//     }
// }

impl MultiErrorHandler {}

pub struct MultiErrorHandlerIndex {
    error_idx: usize,
    slice_idx: usize,
}

impl ErrorHandler for MultiErrorHandler {
    type Indexer = MultiErrorHandlerIndex;

    fn new(start_pos: usize) -> Self {
        Self {
            best_failure_slice: (0, 0),
            slice_stack: vec![(start_pos, start_pos)],
            errors: Vec::new(),
            errors_at_match_start: Vec::new(),
        }
    }

    fn register_start(&mut self, pos: usize) -> Self::Indexer {
        self.errors.push(None);
        self.slice_stack.push((pos, pos));
        MultiErrorHandlerIndex {
            error_idx: self.errors.len() - 1,
            slice_idx: self.slice_stack.len() - 1,
        }
    }

    fn register_watermark(&mut self, pos: usize) {
        if let Some(slice) = self.slice_stack.last_mut() {
            slice.1 = slice.1.max(pos);
        }
    }

    fn register_success(&mut self, idx: Self::Indexer) {
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

    fn register_error<L: Display + 'static>(
        &mut self,
        label: L,
        idx: Self::Indexer,
        match_start: usize,
    ) {
        let failure_slice = self.slice_stack[idx.slice_idx];
        let mut error_idx = idx.error_idx;
        self.register_success(idx); // pop the slice stack for this error

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
            self.errors.clear();
            self.errors_at_match_start.clear();
            self.best_failure_slice = failure_slice;
            self.errors.push(None); // placeholder for this error
            error_idx = self.errors.len() - 1; // update idx to point to the new error
        }

        // remove all errors that occur inside this rule that haven't matched a single token
        while !self.errors_at_match_start.is_empty()
            && self.errors_at_match_start.last().unwrap() > &error_idx
        {
            if let Some(pos) = self.errors_at_match_start.pop() {
                self.errors[pos] = None;
            }
        }

        // Now this error is at least as interesting as the best failure, so register it.
        self.errors[error_idx] = Some(Box::new(label));

        // If this error occurs at its match_start, add it to the list of errors at match_start.
        if match_start == failure_slice.0 {
            self.errors_at_match_start.push(error_idx);
        }
    }
    fn to_choice(&mut self) -> ErrorHandlerChoice<'_> {
        ErrorHandlerChoice::Multi(self)
    }
}

/// An extra source-span label to attach to the report, pointing at a
/// different location than the primary error span.
pub struct ExtraLabel {
    pub span: (usize, usize),
    pub message: String,
    pub color: Color,
}

/// All optional annotations that can be attached to a [`ParserError`].
#[derive(Default)]
pub struct ParserErrorAnnotations {
    pub notes: Vec<String>,
    pub help: Vec<String>,
    pub extra_labels: Vec<ExtraLabel>,
}

// ── ParserError ───────────────────────────────────────────────────────────────

pub struct ParserError {
    pub span: (usize, usize),
    pub expected: Vec<String>,
    pub annotations: ParserErrorAnnotations,
}

impl ParserError {
    // ── notes (shown as "note: …" below the report) ──────────────────────────

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.annotations.notes.push(note.into());
        self
    }

    pub fn add_note(&mut self, note: impl Into<String>) -> &mut Self {
        self.annotations.notes.push(note.into());
        self
    }

    // ── help (shown as "help: …" below the report) ───────────────────────────

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.annotations.help.push(help.into());
        self
    }

    pub fn add_help(&mut self, help: impl Into<String>) -> &mut Self {
        self.annotations.help.push(help.into());
        self
    }

    // ── extra labels (additional highlighted spans in the source) ─────────────

    pub fn with_extra_label(
        mut self,
        span: (usize, usize),
        message: impl Into<String>,
        color: Color,
    ) -> Self {
        self.annotations.extra_labels.push(ExtraLabel {
            span,
            message: message.into(),
            color,
        });
        self
    }

    pub fn add_extra_label(
        &mut self,
        span: (usize, usize),
        message: impl Into<String>,
        color: Color,
    ) -> &mut Self {
        self.annotations.extra_labels.push(ExtraLabel {
            span,
            message: message.into(),
            color,
        });
        self
    }

    // ── rendering ─────────────────────────────────────────────────────────────

    pub fn eprint(&self, source_id: &str, source_text: &str) {
        self.build_report(source_id, source_text)
            .finish()
            .eprint((source_id, Source::from(source_text)))
            .unwrap();
    }

    pub fn write(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        self.build_report(source_id, source_text)
            .finish()
            .write((source_id, Source::from(source_text)), sink)
            .unwrap();
    }

    // ── private helpers ───────────────────────────────────────────────────────

    fn main_message(&self, source_text: &str) -> String {
        let found = source_text
            .get(self.span.0..self.span.1)
            .filter(|s| !s.is_empty())
            .unwrap_or(if self.span.0 >= source_text.len() {
                "end of input"
            } else {
                "unknown token"
            });

        match self.expected.len() {
            0 => format!("unexpected '{}'", found),
            1 => format!("expected {} but found '{}'", self.expected[0], found),
            _ => format!(
                "expected one of {} but found '{}'",
                self.expected.join(", "),
                found
            ),
        }
    }

    pub fn build_report<'s>(
        &self,
        source_id: &'s str,
        source_text: &str,
    ) -> ariadne::ReportBuilder<'_, (&'s str, std::ops::Range<usize>)> {
        let span_range = self.span.0..self.span.1;

        let mut report = Report::build(ReportKind::Error, (source_id, span_range.clone()))
            .with_message("Syntax Error")
            .with_label(
                Label::new((source_id, span_range.clone()))
                    .with_message(self.main_message(source_text))
                    .with_color(Color::Red)
                    .with_order(span_range.start as i32),
            );

        for label in &self.annotations.extra_labels {
            report = report.with_label(
                Label::new((source_id, label.span.0..label.span.1))
                    .with_message(label.message.clone())
                    .with_color(label.color)
                    .with_order(label.span.0 as i32),
            );
        }

        for note in &self.annotations.notes {
            report = report.with_note(note.clone());
        }

        for help in &self.annotations.help {
            report = report.with_help(help.clone());
        }

        report
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
    pub fn to_parser_error(&self) -> ParserError {
        let has_errors = self.errors.iter().any(|e| e.is_some());

        let span = if has_errors {
            self.best_failure_slice
        } else {
            // Bottom of the slice stack: the root span seeded by `new()`.
            // Its `.1` is the furthest watermark reached by the whole parse.
            self.slice_stack.first().copied().unwrap_or((0, 0))
        };

        // Deduplicate labels while preserving a deterministic order.
        let expected: Vec<String> = self
            .errors
            .iter()
            .flatten()
            .map(|e| format!("{}", e))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        ParserError {
            span,
            expected,
            annotations: ParserErrorAnnotations::default(),
        }
    }
}

impl Display for ParserError {
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

impl Debug for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserError")
            .field("span", &self.span)
            .field("expected", &self.expected)
            .field("annotations", &self.annotations)
            .finish()
    }
}
