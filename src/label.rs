use std::fmt::Display;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    matcher::{MatchRunner, Matcher, internal::MatcherImpl},
    parser::{Parser, internal::ParserImpl},
};

pub struct Labeled<L, I> {
    label: L,
    inner: I,
}

impl<L, I> Labeled<L, I> {
    pub fn new(label: L, inner: I) -> Self {
        Self { label, inner }
    }
}

impl<Token, MRes, L, I> MatcherImpl<Token, MRes> for Labeled<L, I>
where
    I: Matcher<Token, MRes>,
    L: Display + Clone + 'static,
{
    const CAN_MATCH_DIRECTLY: bool = I::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = I::HAS_PROPERTY;
    const CAN_FAIL: bool = I::CAN_FAIL;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx,
    {
        // let idx = error_handler.register_start(*pos);
        // if runner.run_match(&self.inner, error_handler, pos)? {
        //     error_handler.register_success(idx);
        //     Ok(true)
        // } else {
        //     error_handler.register_failure_with_label(
        //         self.label.clone(),
        //         idx,
        //         runner.get_parser_context().match_start,
        //     );
        //     Ok(false)
        // }
        runner.run_match(&self.inner, error_handler, pos)
    }
    fn maybe_label_internal(&self) -> Option<Box<dyn Display>> {
        Some(Box::new(self.label.clone()))
    }
}

impl<L, I, Token> ParserImpl<Token> for Labeled<L, I>
where
    I: Parser<Token>,
    L: Display + Clone + 'static,
{
    type Output = I::Output;
    const CAN_FAIL: bool = I::CAN_FAIL;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let idx = error_handler.register_start(*pos);
        match self.inner.parse(context, error_handler, pos)? {
            Some(output) => {
                error_handler.register_success(idx);
                Ok(Some(output))
            }
            None => {
                error_handler.register_failure(Some(self.label.clone()), idx, context.match_start);
                Ok(None)
            }
        }
    }
}

pub trait WithLabel
where
    Self: Sized,
{
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

// impl<L, T, D> MaybeLabel<L> for T
// where
//     T: Deref<Target = D> + ?Sized,
//     D: MaybeLabel<L> + ?Sized,
// {
//     fn maybe_label(&self) -> Option<L> {
//         self.deref().maybe_label()
//     }
// }
