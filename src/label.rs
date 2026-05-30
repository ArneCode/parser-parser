//! Attach a displayable label to a [`crate::matcher::Matcher`] or [`crate::parser::Parser`] for richer errors.

use std::fmt::Display;

use crate::{
    context::ParserContext,
    error::{MatcherRunError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher, MatcherCombinator, internal::MatcherImpl},
    parser::{Parser, ParserCombinator, internal::ParserImpl},
};

/// Wraps `inner` and supplies the matcher implementation’s `maybe_label` hook for parse failure registration from `label`.
#[derive(Clone, Debug)]
pub struct Labeled<L, I> {
    label: L,
    inner: I,
}

impl<L, I> ParserCombinator for Labeled<L, I> where I: ParserCombinator {}

impl<L, I> MatcherCombinator for Labeled<L, I> where I: MatcherCombinator {}

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

    #[inline]
    fn match_with_runner<'a, Runner, M: crate::mode::Mode>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        runner.run_match::<_, M, _>(&self.inner, error_handler, input)
    }
    #[inline]
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

    #[inline]
    fn parse<M: crate::mode::Mode>(
        &self,
        context: &mut ParserContext<'src>,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        if !error_handler.is_real() {
            return self.inner.parse::<M>(context, error_handler, input);
        }
        let idx = error_handler.register_start(input.get_pos().into());
        match self.inner.parse::<M>(context, error_handler, input) {
            Ok(Some(output)) => {
                error_handler.register_success(idx);
                Ok(Some(output))
            }
            Ok(None) => {
                error_handler.register_failure(Some(self.label.clone()), idx);
                Ok(None)
            }
            Err(e) => {
                error_handler.register_failure(Some(self.label.clone()), idx);
                Err(e)
            }
        }
    }

    #[inline]
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
    #[inline]
    fn with_label<L>(self, label: L) -> Labeled<L, Self> {
        Labeled::new(label, self)
    }
}
