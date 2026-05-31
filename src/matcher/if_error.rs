//! Matchers and parsers that only participate while real error handling is active.

use std::fmt::Display;

use crate::error::MatcherRunError;
use crate::matcher::internal::MatcherImpl;

#[derive(Debug, Clone)]
/// Wrapper that runs `inner` only during real error collection; otherwise it succeeds immediately.
pub struct IfError<Inner> {
    inner: Inner,
}

#[derive(Debug, Clone)]
/// Wrapper that runs `inner` only during real error collection; otherwise it fails immediately.
pub struct IfErrorElseFail<Inner> {
    inner: Inner,
}

impl<Inner> super::MatcherCombinator for IfError<Inner> where Inner: super::MatcherCombinator {}
impl<Inner> super::MatcherCombinator for IfErrorElseFail<Inner> where Inner: super::MatcherCombinator
{}

impl<Inner> crate::parser::ParserCombinator for IfErrorElseFail<Inner> where
    Inner: crate::parser::ParserCombinator
{
}

impl<Inner> IfError<Inner> {
    /// Wrap `inner`.
    pub fn new(inner: Inner) -> Self {
        Self { inner }
    }
}

impl<Inner> IfErrorElseFail<Inner> {
    /// Wrap `inner`.
    pub fn new(inner: Inner) -> Self {
        Self { inner }
    }
}

impl<'src, Inner, Inp: crate::input::Input<'src>, MRes>
    super::internal::MatcherImpl<'src, Inp, MRes> for IfError<Inner>
where
    Inner: MatcherImpl<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = false;

    #[inline]
    fn match_with_runner<'a, Runner, M: crate::mode::Mode>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl crate::error::error_handler::ErrorHandler,
        input: &mut crate::input::InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: super::MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        if !M::IS_IN_ERROR_RECOVERY {
            return Ok(true);
        }
        self.inner
            .match_with_runner::<Runner, M>(runner, error_handler, input)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.inner.maybe_label()
    }
}

impl<'src, Inner, Inp: crate::input::Input<'src>, MRes>
    super::internal::MatcherImpl<'src, Inp, MRes> for IfErrorElseFail<Inner>
where
    Inner: MatcherImpl<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    #[inline]
    fn match_with_runner<'a, Runner, M: crate::mode::Mode>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl crate::error::error_handler::ErrorHandler,
        input: &mut crate::input::InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: super::MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        if !M::IS_IN_ERROR_RECOVERY {
            return Ok(false);
        }
        self.inner
            .match_with_runner::<Runner, M>(runner, error_handler, input)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.inner.maybe_label()
    }
}

impl<'src, Inner, Inp: crate::input::Input<'src>> crate::parser::internal::ParserImpl<'src, Inp>
    for IfErrorElseFail<Inner>
where
    Inner: crate::parser::Parser<'src, Inp>,
{
    type Output = <Inner as crate::parser::internal::ParserImpl<'src, Inp>>::Output;
    const CAN_FAIL: bool = true;

    #[inline]
    fn parse<M: crate::mode::Mode>(
        &self,
        context: &mut crate::context::ParserContext<'src>,
        error_handler: &mut impl crate::error::error_handler::ErrorHandler,
        input: &mut crate::input::InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, crate::error::MatcherRunError> {
        if !M::IS_IN_ERROR_RECOVERY {
            return Ok(None);
        }
        self.inner.parse::<M>(context, error_handler, input)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.inner.maybe_label()
    }
}

#[cfg_attr(feature = "parser-trace", track_caller)]
/// Run `inner` only when a real error handler is active; otherwise succeed without consuming input.
pub fn if_error<Inner>(inner: Inner) -> IfError<Inner>
where
    Inner: super::MatcherCombinator,
{
    IfError::new(inner)
}

#[cfg_attr(feature = "parser-trace", track_caller)]
/// Run `inner` only when a real error handler is active; otherwise fail without consuming input.
pub fn if_error_else_fail<Inner>(inner: Inner) -> IfErrorElseFail<Inner> {
    IfErrorElseFail::new(inner)
}
