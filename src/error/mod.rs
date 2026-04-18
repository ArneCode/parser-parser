//! User-facing parse errors and pretty-printing via [ariadne](https://docs.rs/ariadne) and [miette](https://docs.rs/miette).

use ariadne::{Color, Label, Report, ReportKind, Source};
use miette::{
    GraphicalReportHandler, LabeledSpan, MietteDiagnostic, NamedSource, Report as MietteReport,
};

pub(crate) mod error_handler;
// ── ParserError ───────────────────────────────────────────────────────────────

/// Reportable parse issue (attached to a source span), as opposed to a propagating [`FurthestFailError`].
pub enum ParserError {
    /// Expected-token style error at the furthest failure point.
    FurthestFail(FurthestFailError),
    /// Synthetic “missing” insertion (e.g. error recovery).
    Missing(MissingError),
}
impl ParserError {
    /// Build an [ariadne](https://docs.rs/ariadne) report for this error.
    pub fn build_report<'s>(
        &self,
        source_id: &'s str,
        source_text: &str,
    ) -> ariadne::ReportBuilder<'_, (&'s str, std::ops::Range<usize>)> {
        match self {
            ParserError::FurthestFail(err) => err.build_report(source_id, source_text),
            ParserError::Missing(err) => err.build_report(source_id),
        }
    }

    /// Print to stderr using each variant’s default renderer.
    pub fn eprint(&self, source_id: &str, source_text: &str) {
        match self {
            ParserError::FurthestFail(err) => err.eprint(source_id, source_text),
            ParserError::Missing(err) => err.eprint(source_id, source_text),
        }
    }
}

/// Extra highlighted region in a diagnostic (secondary label).
pub struct ExtraLabel {
    /// Byte or character span `(start, end)` in the source string.
    pub span: (usize, usize),
    /// Label text shown at `span`.
    pub message: String,
    /// Highlight color for this label.
    pub color: Color,
}

/// All optional annotations that can be attached to a [`ParserError`].
#[derive(Default)]
pub struct ParserErrorAnnotations {
    /// Lines shown as `note:` in the report.
    pub notes: Vec<String>,
    /// Lines shown as `help:` in the report.
    pub help: Vec<String>,
    /// Additional spans to highlight beyond the primary error.
    pub extra_labels: Vec<ExtraLabel>,
}

/// Hard failure: parser could not match expected alternatives at `span`.
pub struct FurthestFailError {
    /// Primary span `(start, end)` for this failure.
    pub span: (usize, usize),
    /// Human-readable “expected …” fragments (joined in the rendered message).
    pub expected: Vec<String>,
    /// Optional notes, help text, and extra labels.
    pub annotations: ParserErrorAnnotations,
}

impl FurthestFailError {
    /// Wrap as [`ParserError::FurthestFail`].
    pub fn as_parser_error(self) -> ParserError {
        ParserError::FurthestFail(self)
    }
    // ── notes (shown as "note: …" below the report) ──────────────────────────

    /// Add a note (builder style).
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.annotations.notes.push(note.into());
        self
    }

    /// Add a note (mutable).
    pub fn add_note(&mut self, note: impl Into<String>) -> &mut Self {
        self.annotations.notes.push(note.into());
        self
    }

    // ── help (shown as "help: …" below the report) ───────────────────────────

    /// Add help text (builder style).
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.annotations.help.push(help.into());
        self
    }

    /// Add help text (mutable).
    pub fn add_help(&mut self, help: impl Into<String>) -> &mut Self {
        self.annotations.help.push(help.into());
        self
    }

    // ── extra labels (additional highlighted spans in the source) ─────────────

    /// Attach an extra labeled span (builder style).
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

    /// Attach an extra labeled span (mutable).
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

    /// Print this furthest-fail error to stderr (ariadne).
    pub fn eprint(&self, source_id: &str, source_text: &str) {
        self.eprint_ariadne(source_id, source_text);
    }

    /// Print using the ariadne backend.
    pub fn eprint_ariadne(&self, source_id: &str, source_text: &str) {
        self.build_report(source_id, source_text)
            .finish()
            .eprint((source_id, Source::from(source_text)))
            .unwrap();
    }

    /// Write an ariadne report to `sink`.
    pub fn write(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        self.write_ariadne(source_id, source_text, sink);
    }

    /// Write an ariadne report to `sink` (explicit backend).
    pub fn write_ariadne(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        self.build_report(source_id, source_text)
            .finish()
            .write((source_id, Source::from(source_text)), sink)
            .unwrap();
    }

    /// Print using the miette graphical renderer.
    pub fn eprint_miette(&self, source_id: &str, source_text: &str) {
        let mut output = String::new();
        self.render_miette_into(&mut output, source_id, source_text);
        eprint!("{}", output);
    }

    /// Write miette report to `sink`.
    pub fn write_miette(&self, source_id: &str, source_text: &str, mut sink: impl std::io::Write) {
        let mut output = String::new();
        self.render_miette_into(&mut output, source_id, source_text);
        sink.write_all(output.as_bytes()).unwrap();
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

    /// Start building an ariadne “Syntax Error” report.
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

    fn render_miette_into(&self, out: &mut String, source_id: &str, source_text: &str) {
        let main_label = LabeledSpan::new_primary_with_span(
            Some(self.main_message(source_text)),
            self.span.0..self.span.1,
        );
        let extra_labels = self
            .annotations
            .extra_labels
            .iter()
            .map(|label| LabeledSpan::at(label.span.0..label.span.1, label.message.clone()));
        let labels: Vec<LabeledSpan> = std::iter::once(main_label).chain(extra_labels).collect();

        let mut diag = MietteDiagnostic::new("Syntax Error").with_labels(labels);
        let combined_help = self
            .annotations
            .notes
            .iter()
            .map(|note| format!("note: {}", note))
            .chain(self.annotations.help.iter().cloned())
            .collect::<Vec<_>>()
            .join("\n");
        if !combined_help.is_empty() {
            diag = diag.with_help(combined_help);
        }

        let report = MietteReport::new(diag)
            .with_source_code(NamedSource::new(source_id, source_text.to_string()));
        let handler = GraphicalReportHandler::new().with_context_lines(1);
        handler.render_report(out, report.as_ref()).unwrap();
    }
}

/// An error indicating that a required token or construct was missing at a certain position.
pub struct MissingError {
    /// Span associated with the missing construct (often empty or a single point).
    pub span: (usize, usize),
    /// Message shown in the “Missing Syntax” report.
    pub message: String,
}

impl MissingError {
    /// Wrap as [`ParserError::Missing`].
    pub fn as_parser_error(self) -> ParserError {
        ParserError::Missing(self)
    }

    /// Print to stderr (ariadne).
    pub fn eprint(&self, source_id: &str, source_text: &str) {
        self.eprint_ariadne(source_id, source_text);
    }

    /// Print with ariadne.
    pub fn eprint_ariadne(&self, source_id: &str, source_text: &str) {
        self.build_report(source_id)
            .finish()
            .eprint((source_id, Source::from(source_text)))
            .unwrap();
    }

    /// Write ariadne report to `sink`.
    pub fn write(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        self.write_ariadne(source_id, source_text, sink);
    }

    /// Write ariadne report to `sink` (explicit).
    pub fn write_ariadne(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        self.build_report(source_id)
            .finish()
            .write((source_id, Source::from(source_text)), sink)
            .unwrap();
    }

    /// Build an ariadne “Missing Syntax” report.
    pub fn build_report<'s>(
        &self,
        source_id: &'s str,
    ) -> ariadne::ReportBuilder<'_, (&'s str, std::ops::Range<usize>)> {
        let span_range = self.span.0..self.span.1;

        Report::build(ReportKind::Error, (source_id, span_range.clone()))
            .with_message("Missing Syntax")
            .with_label(
                Label::new((source_id, span_range.clone()))
                    .with_message(self.message.clone())
                    .with_color(Color::Red)
                    .with_order(span_range.start as i32),
            )
    }
}
