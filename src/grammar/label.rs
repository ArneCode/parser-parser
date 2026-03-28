use std::{marker::PhantomData, ops::Deref};

use crate::grammar::{HasId, IsCheckable, matcher::Matcher, parser::Parser};

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

impl<T, I, L, MContext> Matcher<T, MContext> for Labeled<L, I>
where
    I: Matcher<T, MContext>,
{
    fn match_pattern(&self, context: &mut MContext, pos: &mut usize) -> Result<(), String> {
        self.inner.match_pattern(context, pos)
    }
}

impl<T, L, I, Out> Parser<T> for Labeled<L, I>
where
    I: Parser<T, Output = Out>,
{
    type Output = Out;

    fn parse(
        &self,
        context: std::rc::Rc<super::context::ParserContext<T>>,
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

impl<T, L, I> IsCheckable<T> for Labeled<L, I>
where
    I: IsCheckable<T>,
{
    fn calc_check(&self, context: &super::context::ParserContext<T>, pos: &mut usize) -> bool {
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
