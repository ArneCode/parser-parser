//! User-facing parse errors and pretty-printing via [ariadne](https://docs.rs/ariadne) and [miette](https://docs.rs/miette).

use ariadne::{Color, Label, Report, ReportKind, Source};
use annotate_snippets::{
    AnnotationKind as SnippetAnnotationKind, Level as SnippetLevel, Renderer as SnippetRenderer,
    Snippet,
};
use codespan_reporting::{
    diagnostic::{Diagnostic, Label as CodespanLabel},
    files::SimpleFile,
    term::{Config as CodespanConfig, emit_to_io_write},
};
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
    /// Found token that should not have been present.
    Unwanted(UnwantedError),
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
            ParserError::Unwanted(err) => err.build_report(source_id),
        }
    }

    /// Print to stderr using each variant’s default renderer.
    pub fn eprint(&self, source_id: &str, source_text: &str) {
        match self {
            ParserError::FurthestFail(err) => err.eprint(source_id, source_text),
            ParserError::Missing(err) => err.eprint(source_id, source_text),
            ParserError::Unwanted(err) => err.eprint(source_id, source_text),
        }
    }

    /// Print a single ariadne report containing all supplied parser errors.
    pub fn eprint_many(errors: &[ParserError], source_id: &str, source_text: &str) {
        if errors.is_empty() {
            return;
        }

        Self::build_many_report(errors, source_id, source_text)
            .finish()
            .eprint((source_id, Source::from(source_text)))
            .unwrap();
    }

    /// Write a single ariadne report containing all supplied parser errors.
    pub fn write_many(
        errors: &[ParserError],
        source_id: &str,
        source_text: &str,
        sink: impl std::io::Write,
    ) {
        if errors.is_empty() {
            return;
        }

        Self::build_many_report(errors, source_id, source_text)
            .finish()
            .write((source_id, Source::from(source_text)), sink)
            .unwrap();
    }

    /// Print a single miette report containing all supplied parser errors.
    pub fn eprint_many_miette(errors: &[ParserError], source_id: &str, source_text: &str) {
        if errors.is_empty() {
            return;
        }

        let mut output = String::new();
        Self::render_many_miette_into(errors, &mut output, source_id, source_text);
        eprint!("{}", output);
    }

    /// Write a single miette report containing all supplied parser errors.
    pub fn write_many_miette(
        errors: &[ParserError],
        source_id: &str,
        source_text: &str,
        mut sink: impl std::io::Write,
    ) {
        if errors.is_empty() {
            return;
        }

        let mut output = String::new();
        Self::render_many_miette_into(errors, &mut output, source_id, source_text);
        sink.write_all(output.as_bytes()).unwrap();
    }

    /// Print a single annotate-snippets report containing all supplied parser errors.
    pub fn eprint_many_annotate_snippets(
        errors: &[ParserError],
        source_id: &str,
        source_text: &str,
    ) {
        if errors.is_empty() {
            return;
        }

        let mut output = String::new();
        Self::render_many_annotate_snippets_into(errors, &mut output, source_id, source_text);
        eprint!("{}", output);
    }

    /// Write a single annotate-snippets report containing all supplied parser errors.
    pub fn write_many_annotate_snippets(
        errors: &[ParserError],
        source_id: &str,
        source_text: &str,
        mut sink: impl std::io::Write,
    ) {
        if errors.is_empty() {
            return;
        }

        let mut output = String::new();
        Self::render_many_annotate_snippets_into(errors, &mut output, source_id, source_text);
        sink.write_all(output.as_bytes()).unwrap();
    }

    /// Print a single codespan-reporting report containing all supplied parser errors.
    pub fn eprint_many_codespan(errors: &[ParserError], source_id: &str, source_text: &str) {
        if errors.is_empty() {
            return;
        }

        let mut output = Vec::new();
        Self::render_many_codespan_into(errors, &mut output, source_id, source_text);
        eprint!("{}", String::from_utf8_lossy(&output));
    }

    /// Write a single codespan-reporting report containing all supplied parser errors.
    pub fn write_many_codespan(
        errors: &[ParserError],
        source_id: &str,
        source_text: &str,
        mut sink: impl std::io::Write,
    ) {
        if errors.is_empty() {
            return;
        }

        let mut output = Vec::new();
        Self::render_many_codespan_into(errors, &mut output, source_id, source_text);
        sink.write_all(&output).unwrap();
    }

    fn build_many_report<'s>(
        errors: &[ParserError],
        source_id: &'s str,
        source_text: &str,
    ) -> ariadne::ReportBuilder<'s, (&'s str, std::ops::Range<usize>)> {
        let anchor = errors
            .iter()
            .map(|error| error.span())
            .min_by_key(|(start, _)| *start)
            .unwrap_or((0, 0));

        let anchor_range = Self::normalized_span(anchor, source_text.len());
        let mut report = Report::build(ReportKind::Error, (source_id, anchor_range))
            .with_message("Parse Errors");

        for error in errors {
            let span = Self::normalized_span(error.span(), source_text.len());
            report = report.with_label(
                Label::new((source_id, span.clone()))
                    .with_message(error.main_message(source_text))
                    .with_color(Color::Red)
                    // .with_order(span.start as i32),
            );

            if let ParserError::FurthestFail(furthest_fail) = error {
                for label in &furthest_fail.annotations.extra_labels {
                    let label_span = Self::normalized_span(label.span, source_text.len());
                    report = report.with_label(
                        Label::new((source_id, label_span.clone()))
                            .with_message(label.message.clone())
                            .with_color(label.color)
                            // .with_order(label_span.start as i32),
                    );
                }

                for note in &furthest_fail.annotations.notes {
                    report = report.with_note(note.clone());
                }

                for help in &furthest_fail.annotations.help {
                    report = report.with_help(help.clone());
                }
            }
        }

        report
    }

    fn render_many_miette_into(
        errors: &[ParserError],
        out: &mut String,
        source_id: &str,
        source_text: &str,
    ) {
        let mut labels = Vec::new();
        let mut notes = Vec::new();
        let mut help = Vec::new();

        for error in errors {
            let span = Self::normalized_span(error.span(), source_text.len());
            if labels.is_empty() {
                labels.push(LabeledSpan::new_primary_with_span(
                    Some(error.main_message(source_text)),
                    span.clone(),
                ));
            } else {
                labels.push(LabeledSpan::at(span.clone(), error.main_message(source_text)));
            }

            if let ParserError::FurthestFail(furthest_fail) = error {
                for label in &furthest_fail.annotations.extra_labels {
                    let label_span = Self::normalized_span(label.span, source_text.len());
                    labels.push(LabeledSpan::at(label_span, label.message.clone()));
                }
                notes.extend(furthest_fail.annotations.notes.iter().cloned());
                help.extend(furthest_fail.annotations.help.iter().cloned());
            }
        }

        let mut diag = MietteDiagnostic::new("Parse Errors").with_labels(labels);
        let combined_help = notes
            .into_iter()
            .map(|note| format!("note: {}", note))
            .chain(help)
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

    fn render_many_annotate_snippets_into(
        errors: &[ParserError],
        out: &mut String,
        source_id: &str,
        source_text: &str,
    ) {
        let mut snippet = Snippet::source(source_text).path(source_id).line_start(1);
        let mut notes = Vec::new();
        let mut help = Vec::new();
        for (idx, error) in errors.iter().enumerate() {
            let span = Self::normalized_span(error.span(), source_text.len());
            let kind = if idx == 0 {
                SnippetAnnotationKind::Primary
            } else {
                SnippetAnnotationKind::Context
            };
            snippet = snippet.annotation(
                kind.span(span).label(error.main_message(source_text)),
            );

            if let ParserError::FurthestFail(furthest_fail) = error {
                for label in &furthest_fail.annotations.extra_labels {
                    let label_span = Self::normalized_span(label.span, source_text.len());
                    snippet = snippet.annotation(
                        SnippetAnnotationKind::Context
                            .span(label_span)
                            .label(label.message.clone()),
                    );
                }
                notes.extend(furthest_fail.annotations.notes.iter().cloned());
                help.extend(furthest_fail.annotations.help.iter().cloned());
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

    fn render_many_codespan_into(
        errors: &[ParserError],
        out: &mut Vec<u8>,
        source_id: &str,
        source_text: &str,
    ) {
        let mut labels = Vec::new();
        let mut notes = Vec::new();

        for (idx, error) in errors.iter().enumerate() {
            let span = Self::normalized_span(error.span(), source_text.len());
            let range = span.start..span.end;
            let label = if idx == 0 {
                CodespanLabel::primary((), range).with_message(error.main_message(source_text))
            } else {
                CodespanLabel::secondary((), range).with_message(error.main_message(source_text))
            };
            labels.push(label);

            if let ParserError::FurthestFail(furthest_fail) = error {
                for label in &furthest_fail.annotations.extra_labels {
                    let label_span = Self::normalized_span(label.span, source_text.len());
                    labels.push(
                        CodespanLabel::secondary((), label_span.start..label_span.end)
                            .with_message(label.message.clone()),
                    );
                }
                notes.extend(
                    furthest_fail
                        .annotations
                        .notes
                        .iter()
                        .map(|note| format!("note: {}", note)),
                );
                notes.extend(furthest_fail.annotations.help.iter().cloned());
            }
        }

        let diagnostic = Diagnostic::error()
            .with_message("Parse Errors")
            .with_labels(labels)
            .with_notes(notes);
        let file = SimpleFile::new(source_id, source_text);
        let config = CodespanConfig::default();
        emit_to_io_write(out, &config, &file, &diagnostic).unwrap();
    }

    fn span(&self) -> (usize, usize) {
        match self {
            ParserError::FurthestFail(error) => error.span,
            ParserError::Missing(error) => error.span,
            ParserError::Unwanted(error) => error.span,
        }
    }

    fn main_message(&self, source_text: &str) -> String {
        match self {
            ParserError::FurthestFail(error) => error.main_message(source_text),
            ParserError::Missing(error) => error.message.clone(),
            ParserError::Unwanted(error) => error.message.clone(),
        }
    }

    fn normalized_span(span: (usize, usize), source_len: usize) -> std::ops::Range<usize> {
        let start = span.0.min(source_len);
        let end = span.1.min(source_len);

        start..end
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

pub struct UnwantedError {
    /// Span associated with the unwanted construct (often empty or a single point).
    pub span: (usize, usize),
    /// Message shown in the “Unwanted Syntax” report.
    pub message: String,
}

impl UnwantedError {
    /// Wrap as [`ParserError::Unwanted`].
    pub fn as_parser_error(self) -> ParserError {
        ParserError::Unwanted(self)
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

    /// Build an ariadne “Unwanted Syntax” report.
    pub fn build_report<'s>(
        &self,
        source_id: &'s str,
    ) -> ariadne::ReportBuilder<'_, (&'s str, std::ops::Range<usize>)> {
        let span_range = self.span.0..self.span.1;

        Report::build(ReportKind::Error, (source_id, span_range.clone()))
            .with_message("Unwanted Syntax")
            .with_label(
                Label::new((source_id, span_range.clone()))
                    .with_message(self.message.clone())
                    .with_color(Color::Red)
                    .with_order(span_range.start as i32),
            )
    }
}
