//! Matchers that emit [`crate::error::InlineError`] when an inner pattern matches / does not match.
//!
//! [`ErrIfMatchedMatcher`] records diagnostics by pushing onto the parse context’s error stack
//! (`ParserContext::push_stack_error`, crate-private) whenever the inner pattern matches; the matcher
//! runner truncates the error stack on failed branches so exploratory matches do not leak stale errors.

use std::fmt;

use crate::{
    error::{BuildInlineError, MatchDiagCtx, MatcherRunError, ParserError},
    input::{Input, InputStream},
    matcher::{Matcher, MatcherCombinator, internal::MatcherImpl},
    parser::capture::MatchResult,
};

#[derive(Clone)]
/// Matcher wrapper that emits a diagnostic when `inner` does not match.
pub struct ErrIfNoMatchMatcher<Inner, F> {
    /// Wrapped matcher.
    pub inner: Inner,
    /// Diagnostic factory used when `inner` does not match.
    pub factory: F,
}

impl<Inner, F> ErrIfNoMatchMatcher<Inner, F> {
    /// Wrap `inner` and `factory`.
    pub fn new(inner: Inner, factory: F) -> Self {
        Self { inner, factory }
    }
}

impl<Inner, F> fmt::Debug for ErrIfNoMatchMatcher<Inner, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ErrIfNoMatchMatcher")
            .finish_non_exhaustive()
    }
}

impl<Inner, F> MatcherCombinator for ErrIfNoMatchMatcher<Inner, F> where Inner: MatcherCombinator {}

impl<'src, Inp: Input<'src>, MRes, Inner, F> MatcherImpl<'src, Inp, MRes>
    for ErrIfNoMatchMatcher<Inner, F>
where
    Inner: Matcher<'src, Inp, MRes>,
    F: BuildInlineError<MRes>,
    Inp: Input<'src>,
    MRes: MatchResult,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = true;

    #[inline]
    fn match_with_runner<'a, Runner, M: crate::mode::Mode>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl crate::error::error_handler::ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: super::MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        let start_pos: usize = input.get_pos().into();
        match runner.run_match::<_, M, _>(&self.inner, error_handler, input)? {
            true => Ok(true),
            false => {
                if M::IS_IN_ERROR_RECOVERY {
                    let ctx = MatchDiagCtx::insertion_point(start_pos);
                    let err =
                        runner.with_snapshot(|snap| self.factory.build_inline_error(ctx, snap));
                    runner
                        .get_parser_context()
                        .push_stack_error(ParserError::Inline(err));
                    Ok(true)
                } else {
                    Err(MatcherRunError::RetryRerunNeeded)
                }
            }
        }
    }
}

#[derive(Clone)]
/// Matcher wrapper that emits a diagnostic when `inner` matches.
pub struct ErrIfMatchedMatcher<Inner, F> {
    /// Wrapped matcher.
    pub inner: Inner,
    /// Diagnostic factory used when `inner` matches.
    pub factory: F,
}

impl<Inner, F> ErrIfMatchedMatcher<Inner, F> {
    /// Wrap `inner` and `factory`.
    pub fn new(inner: Inner, factory: F) -> Self {
        Self { inner, factory }
    }
}

impl<Inner, F> fmt::Debug for ErrIfMatchedMatcher<Inner, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ErrIfMatchedMatcher")
            .finish_non_exhaustive()
    }
}

impl<Inner, F> MatcherCombinator for ErrIfMatchedMatcher<Inner, F> where Inner: MatcherCombinator {}

impl<'src, Inp: Input<'src>, MRes, Inner, F> MatcherImpl<'src, Inp, MRes>
    for ErrIfMatchedMatcher<Inner, F>
where
    Inner: Matcher<'src, Inp, MRes>,
    F: BuildInlineError<MRes>,
    Inp: Input<'src>,
    MRes: MatchResult,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    #[inline]
    fn match_with_runner<'a, Runner, M: crate::mode::Mode>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl crate::error::error_handler::ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: super::MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        let start_pos: usize = input.get_pos().into();
        let matched = runner.run_match::<_, M, _>(&self.inner, error_handler, input)?;
        if matched {
            let end_pos: usize = input.get_pos().into();
            let ctx = MatchDiagCtx {
                start: start_pos,
                end: end_pos,
            };
            let err = runner.with_snapshot(|snap| self.factory.build_inline_error(ctx, snap));
            runner
                .get_parser_context()
                .push_stack_error(ParserError::Inline(err));
        }
        Ok(matched)
    }
}

/// Build an [`ErrIfNoMatchMatcher`] from `inner` and `factory`.
pub fn err_if_no_match<Inner, F>(inner: Inner, factory: F) -> ErrIfNoMatchMatcher<Inner, F>
where
    Inner: super::MatcherCombinator,
{
    ErrIfNoMatchMatcher::new(inner, factory)
}

/// Convenience wrapper for missing-syntax diagnostics built on [`err_if_no_match`].
pub fn try_insert_if_missing<Inner>(
    inner: Inner,
    message: impl Into<String>,
) -> ErrIfNoMatchMatcher<Inner, crate::error::MissingSyntax>
where
    Inner: super::MatcherCombinator,
{
    ErrIfNoMatchMatcher::new(inner, crate::error::MissingSyntax(message.into()))
}

/// Build an [`ErrIfMatchedMatcher`] from `inner` and `factory`.
pub fn err_if_matched<Inner, F>(inner: Inner, factory: F) -> ErrIfMatchedMatcher<Inner, F>
where
    Inner: super::MatcherCombinator,
{
    ErrIfMatchedMatcher::new(inner, factory)
}

/// Convenience wrapper for unwanted-syntax diagnostics built on [`err_if_matched`].
pub fn unwanted<Inner>(
    inner: Inner,
    message: impl Into<String>,
) -> ErrIfMatchedMatcher<Inner, crate::error::UnwantedSyntax>
where
    Inner: super::MatcherCombinator,
{
    ErrIfMatchedMatcher::new(inner, crate::error::UnwantedSyntax(message.into()))
}
