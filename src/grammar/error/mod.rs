use ariadne::{Color, Label, Report, ReportKind, Source};
use miette::{
    GraphicalReportHandler, LabeledSpan, MietteDiagnostic, NamedSource, Report as MietteReport,
};

pub mod error_handler;
// ── ParserError ───────────────────────────────────────────────────────────────
pub enum ParserError {
    FurthestFail(FurthestFailError),
    Missing(MissingError),
}
impl ParserError {
    // as ariadne report
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

    pub fn eprint(&self, source_id: &str, source_text: &str) {
        match self {
            ParserError::FurthestFail(err) => err.eprint(source_id, source_text),
            ParserError::Missing(err) => err.eprint(source_id, source_text),
        }
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
pub struct FurthestFailError {
    pub span: (usize, usize),
    pub expected: Vec<String>,
    pub annotations: ParserErrorAnnotations,
}

impl FurthestFailError {
    pub fn as_parser_error(self) -> ParserError {
        ParserError::FurthestFail(self)
    }
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
        self.eprint_ariadne(source_id, source_text);
    }

    pub fn eprint_ariadne(&self, source_id: &str, source_text: &str) {
        self.build_report(source_id, source_text)
            .finish()
            .eprint((source_id, Source::from(source_text)))
            .unwrap();
    }

    pub fn write(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        self.write_ariadne(source_id, source_text, sink);
    }

    pub fn write_ariadne(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        self.build_report(source_id, source_text)
            .finish()
            .write((source_id, Source::from(source_text)), sink)
            .unwrap();
    }

    pub fn eprint_miette(&self, source_id: &str, source_text: &str) {
        let mut output = String::new();
        self.render_miette_into(&mut output, source_id, source_text);
        eprint!("{}", output);
    }

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
    pub span: (usize, usize),
    pub message: String,
}

impl MissingError {
    pub fn as_parser_error(self) -> ParserError {
        ParserError::Missing(self)
    }

    pub fn eprint(&self, source_id: &str, source_text: &str) {
        self.eprint_ariadne(source_id, source_text);
    }

    pub fn eprint_ariadne(&self, source_id: &str, source_text: &str) {
        self.build_report(source_id)
            .finish()
            .eprint((source_id, Source::from(source_text)))
            .unwrap();
    }

    pub fn write(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        self.write_ariadne(source_id, source_text, sink);
    }

    pub fn write_ariadne(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        self.build_report(source_id)
            .finish()
            .write((source_id, Source::from(source_text)), sink)
            .unwrap();
    }

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
