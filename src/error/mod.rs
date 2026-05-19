//! Parse errors and diagnostics.
//!
//! # Types
//!
//! - [`FurthestFailError`]: expected-token style failure at a span (top-level `Err` from whole-input
//!   parse, or embedded in [`ParserError::FurthestFail`] when recovered).
//! - [`ParserError`]: user-facing issues collected during a parse (`FurthestFail` or [`InlineError`]).
//! - [`MatcherRunError`]: internal runner signal (`FurthestFail` or `RetryRerunNeeded`); library users
//!   usually see the mapped [`FurthestFailError`] instead.
//!
//! # Rendering
//!
//! [`ParserError`], [`FurthestFailError`], and [`InlineError`] implement [`std::fmt::Display`] for plain
//! text (span positions and messages; no source snippet).
//!
//! With the **`annotate-snippets`** feature, use [`ParserError::eprint`] / [`FurthestFailError::eprint`]
//! for terminal output with source excerpts.
//!
//! # Guides
//!
//! See [`crate::guide::errors_and_recovery`] for soft vs hard failure, [`commit_on`](crate::matcher::commit_on),
//! recovery (`recover_with`), and inline diagnostics.

mod inline_error;
#[cfg(feature = "annotate-snippets")]
mod render_annotate;

pub use inline_error::{
    AnnotationKind, BuildInlineError, ClosureBuild, DiagnosticAnnotation,
    InlineError, MatchDiagCtx, MissingSyntax, SnapshotFactory, UnwantedSyntax, ctx_factory,
};

pub(crate) mod error_handler;

// ── ParserError ───────────────────────────────────────────────────────────────

/// Reportable parse issue (attached to a source span), as opposed to a propagating [`FurthestFailError`].
pub enum ParserError {
    /// Expected-token style error at the furthest failure point.
    FurthestFail(FurthestFailError),
    /// User-defined inline diagnostic (from [`crate::matcher::err_if_no_match`], etc.).
    Inline(InlineError),
}

impl ParserError {
    /// Primary span used for sorting / anchoring in multi-error reports.
    #[inline]
    pub fn span(&self) -> (usize, usize) {
        match self {
            ParserError::FurthestFail(error) => error.span,
            ParserError::Inline(error) => error.reporting_span(),
        }
    }

    #[cfg(feature = "annotate-snippets")]
    /// Print this error to stderr (requires **`annotate-snippets`** feature).
    pub fn eprint(&self, source_id: &str, source_text: &str) {
        let mut out = String::new();
        self.render_into(&mut out, source_id, source_text);
        eprint!("{}", out);
    }

    #[cfg(feature = "annotate-snippets")]
    /// Write this error to `sink` (requires **`annotate-snippets`** feature).
    pub fn write(&self, source_id: &str, source_text: &str, mut sink: impl std::io::Write) {
        let mut out = String::new();
        self.render_into(&mut out, source_id, source_text);
        sink.write_all(out.as_bytes()).unwrap();
    }

    #[cfg(feature = "annotate-snippets")]
    fn render_into(&self, out: &mut String, source_id: &str, source_text: &str) {
        render_annotate::render_errors_slice_into(
            std::slice::from_ref(self),
            out,
            source_id,
            source_text,
        );
    }

    #[cfg(feature = "annotate-snippets")]
    /// Print all errors to stderr (requires **`annotate-snippets`** feature).
    pub fn eprint_many(errors: &[ParserError], source_id: &str, source_text: &str) {
        if errors.is_empty() {
            return;
        }
        let mut out = String::new();
        render_annotate::render_errors_slice_into(errors, &mut out, source_id, source_text);
        eprint!("{}", out);
    }

    #[cfg(feature = "annotate-snippets")]
    /// Write all errors to `sink` (requires **`annotate-snippets`** feature).
    pub fn write_many(
        errors: &[ParserError],
        source_id: &str,
        source_text: &str,
        mut sink: impl std::io::Write,
    ) {
        if errors.is_empty() {
            return;
        }
        let mut out = String::new();
        render_annotate::render_errors_slice_into(errors, &mut out, source_id, source_text);
        sink.write_all(out.as_bytes()).unwrap();
    }
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::FurthestFail(e) => e.fmt(f),
            ParserError::Inline(e) => e.fmt(f),
        }
    }
}

/// Hard failure: parser could not match expected alternatives at `span`.
pub struct FurthestFailError {
    /// Primary span `(start, end)` for this failure.
    pub span: (usize, usize),
    /// Human-readable “expected …” fragments (joined in the rendered message).
    pub expected: Vec<String>,
    /// Extra labeled spans that provide context for the failure.
    pub annotations: Vec<DiagnosticAnnotation>,
    /// Additional explanatory notes shown with the diagnostic.
    pub notes: Vec<String>,
    /// Suggested fixes or next steps shown with the diagnostic.
    pub helps: Vec<String>,
}

impl FurthestFailError {
    /// Wrap as [`ParserError::FurthestFail`].
    #[inline]
    pub fn as_parser_error(self) -> ParserError {
        ParserError::FurthestFail(self)
    }

    /// Return a copy of this error with `span` replaced.
    pub fn with_span(mut self, span: (usize, usize)) -> Self {
        self.span = span;
        self
    }

    /// Replace the primary span in place.
    pub fn set_span(&mut self, span: (usize, usize)) -> &mut Self {
        self.span = span;
        self
    }

    /// Return a copy of this error with an additional note.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add a note in place.
    pub fn add_note(&mut self, note: impl Into<String>) -> &mut Self {
        self.notes.push(note.into());
        self
    }

    /// Return a copy of this error with an additional help message.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.helps.push(help.into());
        self
    }

    /// Add a help message in place.
    pub fn add_help(&mut self, help: impl Into<String>) -> &mut Self {
        self.helps.push(help.into());
        self
    }

    /// Return a copy of this error with an additional annotation.
    pub fn with_annotation(
        mut self,
        span: (usize, usize),
        message: impl Into<String>,
        kind: AnnotationKind,
    ) -> Self {
        self.annotations.push(DiagnosticAnnotation {
            span,
            message: message.into(),
            kind,
        });
        self
    }

    /// Add an annotation in place.
    pub fn add_annotation(
        &mut self,
        span: (usize, usize),
        message: impl Into<String>,
        kind: AnnotationKind,
    ) -> &mut Self {
        self.annotations.push(DiagnosticAnnotation {
            span,
            message: message.into(),
            kind,
        });
        self
    }

    #[cfg(feature = "annotate-snippets")]
    /// Print to stderr (requires **`annotate-snippets`** feature).
    pub fn eprint(&self, source_id: &str, source_text: &str) {
        ParserError::FurthestFail(self.clone()).eprint(source_id, source_text);
    }

    #[cfg(feature = "annotate-snippets")]
    /// Write to `sink` (requires **`annotate-snippets`** feature).
    pub fn write(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        ParserError::FurthestFail(self.clone()).write(source_id, source_text, sink);
    }

    #[cfg(feature = "annotate-snippets")]
    pub(crate) fn main_message(&self, source_text: &str) -> String {
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
}

impl Clone for FurthestFailError {
    fn clone(&self) -> Self {
        Self {
            span: self.span,
            expected: self.expected.clone(),
            annotations: self.annotations.clone(),
            notes: self.notes.clone(),
            helps: self.helps.clone(),
        }
    }
}

impl From<InlineError> for ParserError {
    fn from(value: InlineError) -> Self {
        ParserError::Inline(value)
    }
}

impl std::fmt::Display for FurthestFailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let expected_msg = match self.expected.len() {
            0 => "unexpected token".to_string(),
            1 => format!("expected {}", self.expected[0]),
            _ => {
                let mut sorted = self.expected.clone();
                sorted.sort();
                format!("expected one of {}", sorted.join(", "))
            }
        };
        write!(f, "{} at {}..{}", expected_msg, self.span.0, self.span.1)?;
        for ann in &self.annotations {
            write!(
                f,
                "\n  {}..{}: {}",
                ann.span.0, ann.span.1, ann.message
            )?;
        }
        for note in &self.notes {
            write!(f, "\nnote: {note}")?;
        }
        for help in &self.helps {
            write!(f, "\nhelp: {help}")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for FurthestFailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FurthestFailError")
            .field("span", &self.span)
            .field("expected", &self.expected)
            .field("annotations", &self.annotations)
            .field("notes", &self.notes)
            .field("helps", &self.helps)
            .finish()
    }
}

/// Hard failure or control signal from the matcher runner (`MatchRunner::run_match`, crate-private) and matcher composition.
#[derive(Debug)]
pub enum MatcherRunError {
    /// Furthest-failure with full diagnostic payload.
    FurthestFail(FurthestFailError),
    /// Marker: a fast path (e.g. [`crate::matcher::commit_matcher::CommitMatcher`]) requests a rewind
    /// and re-run with a backtracking runner (e.g. capture recovery). No diagnostic was collected for this variant.
    RetryRerunNeeded,
}

impl std::fmt::Display for MatcherRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatcherRunError::FurthestFail(e) => write!(f, "{e}"),
            MatcherRunError::RetryRerunNeeded => write!(f, "retry rerun needed"),
        }
    }
}

impl From<FurthestFailError> for MatcherRunError {
    fn from(value: FurthestFailError) -> Self {
        MatcherRunError::FurthestFail(value)
    }
}

impl MatcherRunError {
    /// Convert to [`FurthestFailError`] for parser APIs. The marker becomes a minimal error at `span`.
    #[inline]
    pub fn into_furthest_fail_for_parser(self, span: (usize, usize)) -> FurthestFailError {
        match self {
            MatcherRunError::FurthestFail(f) => f,
            MatcherRunError::RetryRerunNeeded => FurthestFailError {
                span,
                expected: Vec::new(),
                annotations: Vec::new(),
                notes: vec!["internal: unexpected nested retry marker".into()],
                helps: Vec::new(),
            },
        }
    }
}
