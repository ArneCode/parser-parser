use std::fmt;

use crate::parser::capture::MatchResult;

/// Role of an annotation in a rendered diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationKind {
    /// The main span the diagnostic is about.
    Primary,
    /// A secondary span related to the main issue.
    Secondary,
    /// Contextual span that helps explain the issue.
    Context,
}

/// One labeled span attached to an [`InlineError`] or [`crate::error::FurthestFailError`].
#[derive(Clone, Debug)]
pub struct DiagnosticAnnotation {
    /// Source span covered by the annotation.
    pub span: (usize, usize),
    /// User-facing label shown for this span.
    pub message: String,
    /// How the annotation should be rendered.
    pub kind: AnnotationKind,
}

/// User-facing diagnostic collected during parsing.
#[derive(Clone, Debug)]
pub struct InlineError {
    /// Primary message for the diagnostic.
    pub message: String,
    /// Primary span, if the diagnostic has one.
    pub span: Option<(usize, usize)>,
    /// Additional annotated spans.
    pub annotations: Vec<DiagnosticAnnotation>,
    /// Explanatory notes.
    pub notes: Vec<String>,
    /// Suggested fixes or next steps.
    pub helps: Vec<String>,
}

impl InlineError {
    /// Create a new diagnostic with `message` and no span.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            annotations: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
        }
    }

    /// Create a new diagnostic anchored at `span`.
    pub fn at(span: (usize, usize), message: impl Into<String>) -> Self {
        Self::new(message).with_span(Some(span))
    }

    /// Span used when reporting (falls back to `(0, 0)` if unset).
    pub fn reporting_span(&self) -> (usize, usize) {
        self.span.unwrap_or((0, 0))
    }

    /// Return a copy of this diagnostic with `span` replaced.
    pub fn with_span(mut self, span: Option<(usize, usize)>) -> Self {
        self.span = span;
        self
    }

    /// Replace the primary span in place.
    pub fn set_span(&mut self, span: Option<(usize, usize)>) -> &mut Self {
        self.span = span;
        self
    }

    /// Return a copy of this diagnostic with an additional note.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add a note in place.
    pub fn add_note(&mut self, note: impl Into<String>) -> &mut Self {
        self.notes.push(note.into());
        self
    }

    /// Return a copy of this diagnostic with an additional help message.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.helps.push(help.into());
        self
    }

    /// Add a help message in place.
    pub fn add_help(&mut self, help: impl Into<String>) -> &mut Self {
        self.helps.push(help.into());
        self
    }

    /// Return a copy of this diagnostic with an additional annotation.
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
        crate::error::ParserError::Inline(self.clone()).eprint(source_id, source_text);
    }

    #[cfg(feature = "annotate-snippets")]
    /// Write to `sink` (requires **`annotate-snippets`** feature).
    pub fn write(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        crate::error::ParserError::Inline(self.clone()).write(source_id, source_text, sink);
    }
}

#[derive(Clone, Copy, Debug)]
/// Span context passed to inline diagnostic factories.
pub struct MatchDiagCtx {
    /// Start of the current matched span or insertion point.
    pub start: usize,
    /// End of the current matched span or insertion point.
    pub end: usize,
}

impl MatchDiagCtx {
    /// Return `(start, end)`.
    pub fn span(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    /// Build a zero-width context at `pos`.
    pub fn insertion_point(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }
}

/// Build an [`InlineError`] from a captured-syntax snapshot.
///
/// The trait method names the snapshot lifetime explicitly and adds **`MRes: 'snap`**.
/// This avoids the well-formedness trap of an unbounded universal lifetime over the GAT
/// `MRes::Snapshot<'_>` (which would otherwise force `MRes: 'static`, and transitively
/// `'src: 'static` once the parser is erased).
pub trait BuildInlineError<MRes: MatchResult>: Clone {
    /// Build an [`InlineError`] from the current diagnostic context and snapshot.
    fn build_inline_error<'snap>(
        &self,
        ctx: MatchDiagCtx,
        snapshot: MRes::Snapshot<'snap>,
    ) -> InlineError
    where
        MRes: 'snap;
}

/// Type-enforcing identity helper for plain `|ctx| …` factories.
pub fn ctx_factory<F>(f: F) -> F
where
    F: Fn(MatchDiagCtx) -> InlineError + Clone,
{
    f
}

#[derive(Clone, Debug)]
/// Default builder for missing-syntax diagnostics.
pub struct MissingSyntax(pub String);

impl<MRes: MatchResult> BuildInlineError<MRes> for MissingSyntax {
    fn build_inline_error<'snap>(
        &self,
        ctx: MatchDiagCtx,
        _snapshot: MRes::Snapshot<'snap>,
    ) -> InlineError
    where
        MRes: 'snap,
    {
        InlineError::new(self.0.clone()).with_span(Some((ctx.start, ctx.start)))
    }
}

#[derive(Clone, Debug)]
/// Default builder for unwanted-syntax diagnostics.
pub struct UnwantedSyntax(pub String);

impl<MRes: MatchResult> BuildInlineError<MRes> for UnwantedSyntax {
    fn build_inline_error<'snap>(
        &self,
        ctx: MatchDiagCtx,
        _snapshot: MRes::Snapshot<'snap>,
    ) -> InlineError
    where
        MRes: 'snap,
    {
        InlineError::new(self.0.clone()).with_span(Some((ctx.start, ctx.end)))
    }
}

#[derive(Clone)]
/// Wraps a closure `Fn(MatchDiagCtx) -> InlineError` as a [`BuildInlineError`] implementation.
pub struct ClosureBuild<F>(pub F);

impl<F> fmt::Debug for ClosureBuild<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClosureBuild").finish_non_exhaustive()
    }
}

impl<MRes: MatchResult, F> BuildInlineError<MRes> for ClosureBuild<F>
where
    F: Fn(MatchDiagCtx) -> InlineError + Clone,
{
    fn build_inline_error<'snap>(
        &self,
        ctx: MatchDiagCtx,
        _snapshot: MRes::Snapshot<'snap>,
    ) -> InlineError
    where
        MRes: 'snap,
    {
        (self.0)(ctx)
    }
}

/// Wraps a closure `Fn(&MRes::Snapshot<'a>, MatchDiagCtx) -> InlineError` for [`BuildInlineError`]
/// (typically produced by `use_binds!` inside `capture!`).
///
/// The closure bound goes through the internal `SnapCallable` trait (below) instead of `Fn` directly so that the HRTB
/// over the snapshot lifetime is **conditional on `MRes: 'a`** rather than universal — without
/// this indirection the GAT well-formedness rule forces `MRes: 'static`, which transitively
/// requires `'src: 'static` and breaks any later `.maybe_erase_types()` upstream.
#[derive(Clone)]
pub struct SnapshotFactory<F>(pub F);

impl<F> fmt::Debug for SnapshotFactory<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnapshotFactory").finish_non_exhaustive()
    }
}

/// Helper trait that carries the `MRes: 'a` bound *on the trait itself*.
///
/// `F: for<'a> SnapCallable<'a, MRes>` then effectively means `for<'a where MRes: 'a> Fn(…)` —
/// the universal quantification over `'a` is restricted to lifetimes for which the trait can
/// be satisfied (i.e. those that pass the GAT WF check), so the trick avoids the implicit
/// `MRes: 'static` demand of `for<'a> Fn(&MRes::Snapshot<'a>, …)`.
pub trait SnapCallable<'a, MRes>
where
    MRes: MatchResult + 'a,
{
    /// Build an [`InlineError`] from the borrowed snapshot and context.
    fn call(&self, snap: &MRes::Snapshot<'a>, ctx: MatchDiagCtx) -> InlineError;
}

impl<'a, MRes, F> SnapCallable<'a, MRes> for F
where
    MRes: MatchResult + 'a,
    F: Fn(&MRes::Snapshot<'a>, MatchDiagCtx) -> InlineError,
{
    fn call(&self, snap: &MRes::Snapshot<'a>, ctx: MatchDiagCtx) -> InlineError {
        self(snap, ctx)
    }
}

impl<MRes, F> BuildInlineError<MRes> for SnapshotFactory<F>
where
    MRes: MatchResult,
    F: Clone + for<'a> SnapCallable<'a, MRes>,
{
    fn build_inline_error<'snap>(
        &self,
        ctx: MatchDiagCtx,
        snapshot: MRes::Snapshot<'snap>,
    ) -> InlineError
    where
        MRes: 'snap,
    {
        SnapCallable::<'snap, MRes>::call(&self.0, &snapshot, ctx)
    }
}
