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

use crate::grammar::{HasId, IsCheckable, context::MatcherContext};

pub trait Matcher<T, MContext> {
    fn match_pattern(&self, context: &mut MContext, pos: &mut usize) -> Result<(), String>;
}
pub trait ToMatcher<T, MContext> {
    type MatcherType: Matcher<T, MContext> + HasId + IsCheckable<T>;
    fn to_matcher(&self) -> Self::MatcherType;
}

impl<T, MContext, M> Matcher<T, MContext> for Rc<M>
where
    M: Matcher<T, MContext>,
{
    fn match_pattern(&self, context: &mut MContext, pos: &mut usize) -> Result<(), String> {
        (**self).match_pattern(context, pos)
    }
}
