//! Attach a displayable label to a [`crate::matcher::Matcher`] or [`crate::parser::Parser`] for richer errors.

use std::fmt::Display;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher, MatcherCombinator, internal::MatcherImpl},
    parser::{Parser, ParserCombinator, internal::ParserImpl},
};
#[cfg(feature = "parser-trace")]
use crate::trace::{
    ExplicitMarkerEndOutcome, RuleSourceMetadata, TraceMarkerFailureSnapshot, TraceMarkerPhase,
};
#[cfg(feature = "parser-trace")]
use crate::error::error_handler::ErrorHandlerChoice;

#[cfg(feature = "parser-trace")]
fn trace_failure_snapshot_for_end(
    end_outcome: ExplicitMarkerEndOutcome,
    error_handler: &mut impl ErrorHandler,
    hard_err: Option<&FurthestFailError>,
    input_pos: usize,
) -> Option<TraceMarkerFailureSnapshot> {
    match end_outcome {
        ExplicitMarkerEndOutcome::Success => None,
        ExplicitMarkerEndOutcome::HardError => hard_err.map(TraceMarkerFailureSnapshot::from),
        ExplicitMarkerEndOutcome::SoftFail => match error_handler.to_choice() {
            ErrorHandlerChoice::Multi(m) => Some(TraceMarkerFailureSnapshot::from(&m.to_parser_error())),
            ErrorHandlerChoice::Empty(_) => Some(TraceMarkerFailureSnapshot {
                span_start: input_pos,
                span_end: input_pos,
                expected: vec![],
                summary: "no match (soft fail; no labelled errors recorded for this attempt)".to_string(),
            }),
        },
    }
}

/// Wraps `inner` and supplies [`Matcher::maybe_label`] / parse failure registration from `label`.
#[derive(Clone, Debug)]
pub struct Labeled<L, I> {
    label: L,
    inner: I,
}

impl<L, I> ParserCombinator for Labeled<L, I> where
    I: ParserCombinator,
{
}

impl<L, I> MatcherCombinator for Labeled<L, I> where
    I: MatcherCombinator,
{
}

impl<L, I> Labeled<L, I> {
    /// Pairs a label with an inner parser or matcher.
    pub fn new(label: L, inner: I) -> Self {
        Self { label, inner }
    }
}

impl<'src, Inp: Input<'src>, MRes, L, I> MatcherImpl<'src, Inp, MRes> for Labeled<L, I>
where
    I: Matcher<'src, Inp, MRes>,
    Inp: Input<'src>,
    L: Display + Clone + 'static + std::fmt::Debug,
{
    const CAN_MATCH_DIRECTLY: bool = I::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = I::HAS_PROPERTY;
    const CAN_FAIL: bool = I::CAN_FAIL;

    fn match_with_runner<'a, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        runner.run_match(&self.inner, error_handler, input)
    }
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        Some(Box::new(self.label.clone()))
    }
}

impl<'src, Inp: Input<'src>, L, I> ParserImpl<'src, Inp> for Labeled<L, I>
where
    I: Parser<'src, Inp>,
    Inp: Input<'src>,
    L: Display + Clone + 'static + std::fmt::Debug,
{
    type Output = I::Output;
    const CAN_FAIL: bool = I::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let idx = error_handler.register_start(input.get_pos().into());
        match self.inner.parse(context, error_handler, input)? {
            Some(output) => {
                error_handler.register_success(idx);
                Ok(Some(output))
            }
            None => {
                error_handler.register_failure(Some(self.label.clone()), idx);
                Ok(None)
            }
        }
    }

    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        Some(Box::new(self.label.clone()))
    }
}

/// Extension trait to wrap `self` in [`Labeled`].
pub trait WithLabel
where
    Self: Sized,
{
    /// Same as `Labeled::new(label, self)`.
    fn with_label<L>(self, label: L) -> Labeled<L, Self>;
}

impl<I> WithLabel for I
where
    I: Sized,
{
    fn with_label<L>(self, label: L) -> Labeled<L, Self> {
        Labeled::new(label, self)
    }
}

/// Wraps a parser/matcher in an explicit step marker for tracing.
#[derive(Clone, Debug)]
pub struct Traced<I> {
    inner: I,
    label: Option<String>,
    #[cfg(feature = "parser-trace")]
    source: RuleSourceMetadata,
}

impl<I> ParserCombinator for Traced<I> where I: ParserCombinator {}
impl<I> MatcherCombinator for Traced<I> where I: MatcherCombinator {}

impl<I> Traced<I> {
    #[cfg(feature = "parser-trace")]
    #[track_caller]
    pub fn new(inner: I, label: Option<String>) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            inner,
            label,
            source: RuleSourceMetadata::new(caller.file(), caller.line(), caller.column()),
        }
    }

    #[cfg(not(feature = "parser-trace"))]
    pub fn new(inner: I, label: Option<String>) -> Self {
        Self { inner, label }
    }
}

impl<'src, Inp: Input<'src>, MRes, I> MatcherImpl<'src, Inp, MRes> for Traced<I>
where
    I: Matcher<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = I::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = I::HAS_PROPERTY;
    const CAN_FAIL: bool = I::CAN_FAIL;

    fn match_with_runner<'a, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        #[cfg(feature = "parser-trace")]
        let trace_label = self
            .label
            .clone()
            .or_else(|| self.inner.maybe_label().map(|l| format!("{l}")));
        #[cfg(feature = "parser-trace")]
        let marker_id = {
            let context = runner.get_parser_context();
            let marker_id = context.next_trace_marker_id();
            context.trace_explicit_marker(
                marker_id,
                TraceMarkerPhase::Start,
                input.get_pos().into(),
                trace_label.clone(),
                self.source,
                None,
                None,
                None,
            );
            marker_id
        };
        let matched = runner.run_match(&self.inner, error_handler, input);
        #[cfg(feature = "parser-trace")]
        {
            let end_outcome = match &matched {
                Ok(true) => ExplicitMarkerEndOutcome::Success,
                Ok(false) => ExplicitMarkerEndOutcome::SoftFail,
                Err(_) => ExplicitMarkerEndOutcome::HardError,
            };
            let failure_snapshot = trace_failure_snapshot_for_end(
                end_outcome,
                error_handler,
                matched.as_ref().err(),
                input.get_pos().into(),
            );
            runner.get_parser_context().trace_explicit_marker(
                marker_id,
                TraceMarkerPhase::End,
                input.get_pos().into(),
                trace_label,
                self.source,
                None,
                Some(end_outcome),
                failure_snapshot,
            );
        }
        matched
    }

    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.inner.maybe_label()
    }

}

impl<'src, Inp: Input<'src>, I> ParserImpl<'src, Inp> for Traced<I>
where
    I: Parser<'src, Inp>,
{
    type Output = I::Output;
    const CAN_FAIL: bool = I::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        #[cfg(feature = "parser-trace")]
        let trace_label = self
            .label
            .clone()
            .or_else(|| self.inner.maybe_label().map(|l| format!("{l}")));
        #[cfg(feature = "parser-trace")]
        let marker_id = {
            let marker_id = context.next_trace_marker_id();
            context.trace_explicit_marker(
                marker_id,
                TraceMarkerPhase::Start,
                input.get_pos().into(),
                trace_label.clone(),
                self.source,
                None,
                None,
                None,
            );
            marker_id
        };
        let parsed = self.inner.parse(context, error_handler, input);
        #[cfg(feature = "parser-trace")]
        {
            let end_outcome = match &parsed {
                Ok(Some(_)) => ExplicitMarkerEndOutcome::Success,
                Ok(None) => ExplicitMarkerEndOutcome::SoftFail,
                Err(_) => ExplicitMarkerEndOutcome::HardError,
            };
            let failure_snapshot = trace_failure_snapshot_for_end(
                end_outcome,
                error_handler,
                parsed.as_ref().err(),
                input.get_pos().into(),
            );
            context.trace_explicit_marker(
                marker_id,
                TraceMarkerPhase::End,
                input.get_pos().into(),
                trace_label,
                self.source,
                None,
                Some(end_outcome),
                failure_snapshot,
            );
        }
        parsed
    }

    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.inner.maybe_label()
    }
}

pub trait WithTrace
where
    Self: Sized,
{
    #[cfg_attr(feature = "parser-trace", track_caller)]
    fn trace(self) -> Traced<Self>;
    #[cfg_attr(feature = "parser-trace", track_caller)]
    fn trace_with_label(self, label: impl Into<String>) -> Traced<Self>;
}

impl<I> WithTrace for I
where
    I: Sized,
{
    #[cfg_attr(feature = "parser-trace", track_caller)]
    fn trace(self) -> Traced<Self> {
        Traced::new(self, None)
    }

    #[cfg_attr(feature = "parser-trace", track_caller)]
    fn trace_with_label(self, label: impl Into<String>) -> Traced<Self> {
        Traced::new(self, Some(label.into()))
    }
}
