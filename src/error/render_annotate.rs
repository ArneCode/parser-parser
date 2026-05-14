//! Pretty error rendering via [annotate-snippets](https://docs.rs/annotate-snippets).
//! Compiled only with the `annotate-snippets` feature.

use annotate_snippets::{
    AnnotationKind as SnippetAnnotationKind, Level as SnippetLevel, Renderer as SnippetRenderer,
    Snippet,
};

use super::{AnnotationKind, ParserError};

pub(super) fn render_errors_slice_into(
    errors: &[ParserError],
    out: &mut String,
    source_id: &str,
    source_text: &str,
) {
    let mut snippet = Snippet::source(source_text).path(source_id).line_start(1);
    let mut notes = Vec::new();
    let mut help = Vec::new();

    for (idx, error) in errors.iter().enumerate() {
        let label = match error {
            ParserError::FurthestFail(ff) => ff.main_message(source_text),
            ParserError::Inline(ie) => ie.message.clone(),
        };
        match error {
            ParserError::FurthestFail(ff) => {
                let span = normalized_span(ff.span, source_text.len());
                let kind = if idx == 0 {
                    SnippetAnnotationKind::Primary
                } else {
                    SnippetAnnotationKind::Context
                };
                snippet = snippet.annotation(kind.span(span).label(label));
                for ann in &ff.annotations {
                    let s = normalized_span(ann.span, source_text.len());
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
                let span = normalized_span(ie.reporting_span(), source_text.len());
                let kind = if idx == 0 {
                    SnippetAnnotationKind::Primary
                } else {
                    SnippetAnnotationKind::Context
                };
                snippet = snippet.annotation(kind.span(span).label(label));
                for ann in &ie.annotations {
                    let s = normalized_span(ann.span, source_text.len());
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

fn annotation_kind_to_snippet(kind: AnnotationKind) -> SnippetAnnotationKind {
    match kind {
        AnnotationKind::Primary => SnippetAnnotationKind::Primary,
        AnnotationKind::Secondary | AnnotationKind::Context => SnippetAnnotationKind::Context,
    }
}
