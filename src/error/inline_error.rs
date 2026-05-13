use std::fmt;

use crate::parser::capture::MatchResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationKind {
    Primary,
    Secondary,
    Context,
}

#[derive(Clone, Debug)]
pub struct DiagnosticAnnotation {
    pub span: (usize, usize),
    pub message: String,
    pub kind: AnnotationKind,
}

#[derive(Clone, Debug)]
pub struct InlineError {
    pub message: String,
    pub span: Option<(usize, usize)>,
    pub annotations: Vec<DiagnosticAnnotation>,
    pub notes: Vec<String>,
    pub helps: Vec<String>,
}

impl InlineError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            annotations: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
        }
    }

    pub fn at(span: (usize, usize), message: impl Into<String>) -> Self {
        Self::new(message).with_span(Some(span))
    }

    /// Span used when reporting (falls back to `(0, 0)` if unset).
    pub fn reporting_span(&self) -> (usize, usize) {
        self.span.unwrap_or((0, 0))
    }

    pub fn with_span(mut self, span: Option<(usize, usize)>) -> Self {
        self.span = span;
        self
    }

    pub fn set_span(&mut self, span: Option<(usize, usize)>) -> &mut Self {
        self.span = span;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn add_note(&mut self, note: impl Into<String>) -> &mut Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.helps.push(help.into());
        self
    }

    pub fn add_help(&mut self, help: impl Into<String>) -> &mut Self {
        self.helps.push(help.into());
        self
    }

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

    /// Print to stderr (annotate-snippets).
    pub fn eprint(&self, source_id: &str, source_text: &str) {
        crate::error::ParserError::Inline(self.clone()).eprint(source_id, source_text);
    }

    /// Write to `sink` (annotate-snippets).
    pub fn write(&self, source_id: &str, source_text: &str, sink: impl std::io::Write) {
        crate::error::ParserError::Inline(self.clone()).write(source_id, source_text, sink);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MatchDiagCtx {
    pub start: usize,
    pub end: usize,
}

impl MatchDiagCtx {
    pub fn span(&self) -> (usize, usize) {
        (self.start, self.end)
    }

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
/// The closure bound goes through [`SnapCallable`] instead of `Fn` directly so that the HRTB
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
