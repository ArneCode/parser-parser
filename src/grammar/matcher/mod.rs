pub mod any_token;
pub mod multiple;
pub mod negative_lookahead;
pub mod one_of;
pub mod one_or_more;
pub mod optional;
pub mod parser_matcher;
pub mod positive_lookahead;
pub mod sequence;
pub mod string;
use std::ops::Deref;

use crate::grammar::{context::MatcherContext, error_handler::ErrorHandler};

pub trait Matcher<Token, MatchResult> {
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MatchResult, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String>;
}
pub trait ToMatcher {
    type MatcherType;
    fn to_matcher(&self) -> Self::MatcherType;
}

impl<Outer, Inner, Token, MRes> Matcher<Token, MRes> for Outer
where
    Outer: Deref<Target = Inner>,
    Inner: Matcher<Token, MRes>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        (**self).match_pattern(context, pos)
    }
}
