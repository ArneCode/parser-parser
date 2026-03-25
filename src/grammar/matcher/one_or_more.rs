use crate::grammar::{
    AstNode, Grammar, HasId, IsCheckable, Token,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::Matcher,
};
use std::marker::PhantomData;
pub struct OneOrMore<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    matcher: Match,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Match> OneOrMore<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    pub fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

/// e+  — match one or more repetitions of `matcher`, capturing each occurrence.
pub fn one_or_more<T, N, Match>(matcher: Match) -> OneOrMore<T, N, Match>
where
    T: Token + 'static,
    N: AstNode + ?Sized + 'static,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T> + 'static,
{
    OneOrMore::new(matcher)
}

impl<T, N, Match> HasId for OneOrMore<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, N, Match> IsCheckable<T> for OneOrMore<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        // Must consume at least one token.
        if !self.matcher.check(context, pos) {
            return false;
        }
        // Greedily consume the rest (mirrors Multiple).
        while self.matcher.check(context, pos) {}
        true
    }
}

impl<T, N, Match> Matcher<T> for OneOrMore<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        // First match is mandatory — propagate the error if absent.
        self.matcher.match_pattern(context, pos)?;
        // Remaining matches are optional (same as Multiple).
        while self.matcher.check_no_advance(context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}
