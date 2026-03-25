pub mod any_token;
pub mod multiple;
pub mod negative_lookahead;
pub mod one_of;
pub mod one_or_more;
pub mod optional;
pub mod positive_lookahead;
pub mod sequence;
pub mod string;
use std::rc::Rc;

use crate::grammar::{AstNode, HasId, IsCheckable, Token, context::MatcherContext};

pub trait Matcher<T: Token> {
    type Output: AstNode + ?Sized;
    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String>;
}
pub trait ToMatcher<T: Token, N: AstNode + ?Sized> {
    type MatcherType: Matcher<T, Output = N> + HasId + IsCheckable<T>;
    fn to_matcher(&self) -> Self::MatcherType;
}

// impl Matcher for all Rc<Matcher>
impl<T, N, M> Matcher<T> for Rc<M>
where
    T: Token,
    N: AstNode + ?Sized,
    M: Matcher<T, Output = N>,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        (**self).match_pattern(context, pos)
    }
}
