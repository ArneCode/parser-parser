//! Attach a displayable label to a [`crate::matcher::Matcher`] or [`crate::parser::Parser`] for richer errors.

use std::fmt::Display;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher, internal::MatcherImpl},
    parser::{Parser, internal::ParserImpl},
};

/// Wraps `inner` and supplies [`Matcher::maybe_label`] / parse failure registration from `label`.
pub struct Labeled<L, I> {
    label: L,
    inner: I,
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
    L: Display + Clone + 'static,
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
    L: Display + Clone + 'static,
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
