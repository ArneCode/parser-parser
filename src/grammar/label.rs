use std::fmt::Display;

use crate::grammar::{
    context::ParserContext,
    error_handler::{ErrorHandler, ParserError},
    matcher::{CanImplMatchWithRunner, CanMatchWithRunner, MatchRunner},
    parser::Parser,
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

impl<'a, 'ctx, L, I, Runner> CanImplMatchWithRunner<Runner> for Labeled<L, I>
where
    I: CanMatchWithRunner<Runner>,
    Runner: MatchRunner<'a, 'ctx>,
    L: Display + Clone + 'static,
{
    fn impl_match_with_runner(
        &self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError> {
        let idx = error_handler.register_start(*pos);
        if runner.run_match(&self.inner, error_handler, pos)? {
            error_handler.register_success(idx);
            Ok(true)
        } else {
            error_handler.register_error(
                self.label.clone(),
                idx,
                runner.get_parser_context().match_start,
            );
            Ok(false)
        }
    }
}

impl<'ctx, L, I, Token> Parser<'ctx, Token> for Labeled<L, I>
where
    I: Parser<'ctx, Token>,
    L: Display + Clone + 'static,
{
    type Output = I::Output;

    fn parse(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, ParserError> {
        let idx = error_handler.register_start(*pos);
        match self.inner.parse(context, error_handler, pos)? {
            Some(output) => {
                error_handler.register_success(idx);
                Ok(Some(output))
            }
            None => {
                error_handler.register_error(self.label.clone(), idx, context.match_start);
                Ok(None)
            }
        }
    }
}

// impl<L: Clone, I> MaybeLabel<L> for Labeled<L, I> {
//     fn maybe_label(&self) -> Option<L> {
//         Some(self.label.clone())
//     }
// }

// impl<Inner, Label, Token, MRes> Matcher<Token, MRes> for Labeled<Label, Inner>
// where
//     Inner: Matcher<Token, MRes>,
// {
//     fn match_pattern(
//         &self,
//         context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
//         pos: &mut usize,
//     ) -> Result<(), String> {
//         self.inner.match_pattern(context, pos)
//     }
// }

// impl<L, I, Token> Parser<Token> for Labeled<L, I>
// where
//     I: Parser<Token>,
// {
//     type Output = I::Output;

//     fn parse(
//         &self,
//         context: &mut ParserContext<Token, impl ErrorHandler>,
//         pos: &mut usize,
//     ) -> Result<Self::Output, String> {
//         self.inner.parse(context, pos)
//     }
// }

// impl<L, I> HasId for Labeled<L, I>
// where
//     I: HasId,
// {
//     fn id(&self) -> usize {
//         self.inner.id()
//     }
// }

// impl<L, I, Token> IsCheckable<Token> for Labeled<L, I>
// where
//     I: IsCheckable<Token>,
// {
//     fn calc_check(
//         &self,
//         context: &mut ParserContext<Token, impl ErrorHandler>,
//         pos: &mut usize,
//     ) -> bool {
//         self.inner.calc_check(context, pos)
//     }
// }

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
