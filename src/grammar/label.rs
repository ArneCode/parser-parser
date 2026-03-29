use std::ops::Deref;

use crate::grammar::{
    HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    matcher::Matcher,
    parser::Parser,
};

pub trait MaybeLabel {
    type Label;
    fn maybe_label(&self) -> Option<Self::Label> {
        None
    }
}

pub struct Labeled<L, I> {
    label: L,
    inner: I,
}

impl<L, I> Labeled<L, I> {
    pub fn new(label: L, inner: I) -> Self {
        Self { label, inner }
    }
}

impl<L: Clone, I> MaybeLabel for Labeled<L, I> {
    type Label = L;
    fn maybe_label(&self) -> Option<Self::Label> {
        Some(self.label.clone())
    }
}

impl<Inner, Label, Token, MRes> Matcher<Token, MRes> for Labeled<Label, Inner>
where
    Inner: Matcher<Token, MRes>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        self.inner.match_pattern(context, pos)
    }
}

impl<L, I, Token> Parser<Token> for Labeled<L, I>
where
    I: Parser<Token>,
{
    type Output = I::Output;

    fn parse(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        self.inner.parse(context, pos)
    }
}

impl<L, I> HasId for Labeled<L, I>
where
    I: HasId,
{
    fn id(&self) -> usize {
        self.inner.id()
    }
}

impl<L, I, Token> IsCheckable<Token> for Labeled<L, I>
where
    I: IsCheckable<Token>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        self.inner.calc_check(context, pos)
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

impl<L, T, D> MaybeLabel for T
where
    T: Deref<Target = D>,
    D: MaybeLabel<Label = L>,
{
    type Label = L;
    fn maybe_label(&self) -> Option<Self::Label> {
        self.deref().maybe_label()
    }
}
