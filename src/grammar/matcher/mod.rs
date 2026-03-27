pub mod any_token;
pub mod multiple;
pub mod negative_lookahead;
pub mod one_of;
pub mod one_or_more;
pub mod optional;
pub mod positive_lookahead;
pub mod sequence;
pub mod string;
use std::ops::Deref;

pub trait Matcher<T, MContext> {
    fn match_pattern(&self, context: &mut MContext, pos: &mut usize) -> Result<(), String>;
}
pub trait ToMatcher<T> {
    type MatcherType;
    fn to_matcher(&self) -> Self::MatcherType;
}

impl<Token, MContext, M, T> Matcher<Token, MContext> for T
where
    T: Deref<Target = M>,
    M: Matcher<Token, MContext>,
{
    fn match_pattern(&self, context: &mut MContext, pos: &mut usize) -> Result<(), String> {
        (**self).match_pattern(context, pos)
    }
}
