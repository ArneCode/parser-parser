//! Explicit `.trace()` combinators for marker-first tracing.

use std::fmt::Display;

use crate::{
    context::ParserContext,
    error::{MatcherRunError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher, MatcherCombinator, internal::MatcherImpl},
    parser::{Parser, ParserCombinator, internal::ParserImpl},
};
#[cfg(feature = "parser-trace")]
use crate::error::error_handler::ErrorHandlerChoice;

#[cfg(feature = "parser-trace")]
use super::{
    ExplicitMarkerEndOutcome, RuleSourceMetadata, TraceMarkerFailureSnapshot, TraceMarkerPhase,
};

#[cfg(feature = "parser-trace")]
fn trace_failure_snapshot_for_end(
    end_outcome: ExplicitMarkerEndOutcome,
    error_handler: &mut impl ErrorHandler,
    hard_err: Option<TraceMarkerFailureSnapshot>,
    input_pos: usize,
) -> Option<TraceMarkerFailureSnapshot> {
    match end_outcome {
        ExplicitMarkerEndOutcome::Success => None,
        ExplicitMarkerEndOutcome::HardError => hard_err,
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

/// Wraps a parser/matcher in an explicit step marker for tracing.
#[derive(Clone, Debug)]
pub struct Traced<I> {
    inner: I,
    #[cfg_attr(not(feature = "parser-trace"), allow(dead_code))]
    label: Option<String>,
    #[cfg(feature = "parser-trace")]
    source: RuleSourceMetadata,
}

impl<I> ParserCombinator for Traced<I> where I: ParserCombinator {}
impl<I> MatcherCombinator for Traced<I> where I: MatcherCombinator {}

impl<I> Traced<I> {
    #[cfg(feature = "parser-trace")]
    #[track_caller]
    /// Wrap `inner` in an explicit trace marker, optionally overriding the displayed label.
    pub fn new(inner: I, label: Option<String>) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            inner,
            label,
            source: RuleSourceMetadata::new(caller.file(), caller.line(), caller.column()),
        }
    }

    #[cfg(not(feature = "parser-trace"))]
    /// Wrap `inner` in an explicit trace marker, optionally overriding the displayed label.
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

    #[inline]
    fn match_with_runner<'a, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
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
                matched.as_ref().err().map(TraceMarkerFailureSnapshot::from),
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

    #[inline]
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

    #[inline]
    fn parse(
        &self,
        context: &mut ParserContext<'src>,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
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
                parsed.as_ref().err().map(TraceMarkerFailureSnapshot::from),
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

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.inner.maybe_label()
    }
}

/// Extension methods for adding explicit trace markers to parsers and matchers.
pub trait WithTrace
where
    Self: Sized,
{
    #[cfg_attr(feature = "parser-trace", track_caller)]
    /// Trace this parser or matcher using its existing label (if any).
    fn trace(self) -> Traced<Self>;
    #[cfg_attr(feature = "parser-trace", track_caller)]
    /// Trace this parser or matcher with an explicit `label`.
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
