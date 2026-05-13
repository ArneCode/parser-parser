//! User-facing parse errors and pretty-printing via [annotate-snippets](https://docs.rs/annotate-snippets).

mod inline_error;
pub use inline_error::{
    AnnotationKind, BuildInlineError, ClosureBuild, DiagnosticAnnotation, InlineError,
    MatchDiagCtx, MissingSyntax, SnapshotFactory, UnwantedSyntax, ctx_factory,
};

use annotate_snippets::{
    AnnotationKind as SnippetAnnotationKind, Level as SnippetLevel, Renderer as SnippetRenderer,
    Snippet,
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
    pub fn span(&self) -> (usize, usize) {
        match self {
            ParserError::FurthestFail(error) => error.span,
            ParserError::Inline(error) => error.reporting_span(),
        }
    }

    fn main_message(&self, source_text: &str) -> String {
        match self {
            ParserError::FurthestFail(error) => error.main_message(source_text),
            ParserError::Inline(error) => error.message.clone(),
        }
    }

    /// Print this error to stderr (annotate-snippets).
    pub fn eprint(&self, source_id: &str, source_text: &str) {
        let mut out = String::new();
        self.render_into(&mut out, source_id, source_text);
        eprint!("{}", out);
    }

    /// Write this error to `sink` (annotate-snippets).
    pub fn write(&self, source_id: &str, source_text: &str, mut sink: impl std::io::Write) {
        let mut out = String::new();
        self.render_into(&mut out, source_id, source_text);
        sink.write_all(out.as_bytes()).unwrap();
    }

    fn render_into(&self, out: &mut String, source_id: &str, source_text: &str) {
        Self::render_slice_into(std::slice::from_ref(self), out, source_id, source_text);
    }

    /// Print all errors to stderr (annotate-snippets).
    pub fn eprint_many(errors: &[ParserError], source_id: &str, source_text: &str) {
        if errors.is_empty() {
            return;
        }
        let mut out = String::new();
        Self::render_slice_into(errors, &mut out, source_id, source_text);
        eprint!("{}", out);
    }

    /// Write all errors to `sink` (annotate-snippets).
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
        Self::render_slice_into(errors, &mut out, source_id, source_text);
        sink.write_all(out.as_bytes()).unwrap();
    }

    fn render_slice_into(errors: &[ParserError], out: &mut String, source_id: &str, source_text: &str) {
        let mut snippet = Snippet::source(source_text).path(source_id).line_start(1);
        let mut notes = Vec::new();
        let mut help = Vec::new();

        for (idx, error) in errors.iter().enumerate() {
            let label = error.main_message(source_text);
            match error {
                ParserError::FurthestFail(ff) => {
                    let span = Self::normalized_span(ff.span, source_text.len());
                    let kind = if idx == 0 {
                        SnippetAnnotationKind::Primary
                    } else {
                        SnippetAnnotationKind::Context
                    };
                    snippet = snippet.annotation(kind.span(span).label(label));
                    for ann in &ff.annotations {
                        let s = Self::normalized_span(ann.span, source_text.len());
                        snippet = snippet.annotation(
                            annotation_kind_to_snippet(ann.kind)
                                .span(s)
                                .label(ann.message.clone()),
                        );
                    }
                    notes.extend(ff.notes.iter().cloned());
                    help.extend(ff.helps.iter().cloned());
                }
                ParserError::Inline(ie) => {
                    let span = Self::normalized_span(ie.reporting_span(), source_text.len());
                    let kind = if idx == 0 {
                        SnippetAnnotationKind::Primary
                    } else {
                        SnippetAnnotationKind::Context
                    };
                    snippet = snippet.annotation(kind.span(span).label(label));
                    for ann in &ie.annotations {
                        let s = Self::normalized_span(ann.span, source_text.len());
                        snippet = snippet.annotation(
                            annotation_kind_to_snippet(ann.kind)
                                .span(s)
                                .label(ann.message.clone()),
                        );
                    }
                    notes.extend(ie.notes.iter().cloned());
                    help.extend(ie.helps.iter().cloned());
                }
            }
        }

        let mut group = SnippetLevel::ERROR
            .primary_title("Parse Errors")
            .element(snippet);

        for note in notes {
            group = group.element(SnippetLevel::NOTE.message(note));
        }
        for help_line in help {
            group = group.element(SnippetLevel::HELP.message(help_line));
        }
        let report = [group];

        let rendered = SnippetRenderer::styled().render(&report).to_string();
        out.push_str(&rendered);
        if !rendered.ends_with('\n') {
            out.push('\n');
        }
    }

    fn normalized_span(span: (usize, usize), source_len: usize) -> std::ops::Range<usize> {
        let mut start = span.0.min(source_len);
        let mut end = span.1.min(source_len);

        if end > source_len {
            end = source_len;
        }

        if start == source_len && source_len > 0 {
            start = source_len - 1;
        }

        start..end
    }
}

fn annotation_kind_to_snippet(kind: AnnotationKind) -> SnippetAnnotationKind {
    match kind {
        AnnotationKind::Primary => SnippetAnnotationKind::Primary,
        AnnotationKind::Secondary | AnnotationKind::Context => SnippetAnnotationKind::Context,
    }
}

/// Hard failure: parser could not match expected alternatives at `span`.
pub struct FurthestFailError {
    /// Primary span `(start, end)` for this failure.
    pub span: (usize, usize),
    /// Human-readable “expected …” fragments (joined in the rendered message).
    pub expected: Vec<String>,
    pub annotations: Vec<DiagnosticAnnotation>,
    pub notes: Vec<String>,
    pub helps: Vec<String>,
}

impl FurthestFailError {
    /// Wrap as [`ParserError::FurthestFail`].
    pub fn as_parser_error(self) -> ParserError {
        ParserError::FurthestFail(self)
    }

    pub fn set_span(mut self, span: (usize, usize)) -> Self {
        self.span = span;
        self
    }

    pub fn with_span(self, span: (usize, usize)) -> Self {
        self.set_span(span)
    }

    pub fn set_span_mut(&mut self, span: (usize, usize)) -> &mut Self {
        self.span = span;
        self
    }

    pub fn add_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_note(self, note: impl Into<String>) -> Self {
        self.add_note(note)
    }

    pub fn add_note_mut(&mut self, note: impl Into<String>) -> &mut Self {
        self.notes.push(note.into());
        self
    }

    pub fn add_help(mut self, help: impl Into<String>) -> Self {
        self.helps.push(help.into());
        self
    }

    pub fn with_help(self, help: impl Into<String>) -> Self {
        self.add_help(help)
    }

    pub fn add_help_mut(&mut self, help: impl Into<String>) -> &mut Self {
        self.helps.push(help.into());
        self
    }

    pub fn add_annotation(
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

    pub fn with_annotation(
        self,
        span: (usize, usize),
        message: impl Into<String>,
        kind: AnnotationKind,
    ) -> Self {
        self.add_annotation(span, message, kind)
    }

    pub fn add_annotation_mut(
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

    /// Print to stderr (annotate-snippets).
    pub fn eprint(&self, source_id: &str, source_text: &str) {
        ParserError::FurthestFail(self.clone()).eprint(source_id, source_text);
    }

    /// Write to `sink` (annotate-snippets).
    pub fn write(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        ParserError::FurthestFail(self.clone()).write(source_id, source_text, sink);
    }

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
        write!(f, "{} at {}..{}", expected_msg, self.span.0, self.span.1)
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
