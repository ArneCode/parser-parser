use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::ParserContext,
    get_next_id,
    matcher::Matcher,
};
use std::{marker::PhantomData, ops::Deref};
pub struct OneOrMore<T, MContext, Match> {
    matcher: Match,
    id: usize,
    _phantom: PhantomData<(T, MContext)>,
}

impl<T, MContext, Match> OneOrMore<T, MContext, Match>
where
    Match: Matcher<T, MContext> + HasId + IsCheckable<T>,
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
pub fn one_or_more<T, MContext, Match>(matcher: Match) -> OneOrMore<T, MContext, Match>
where
    Match: Matcher<T, MContext> + HasId + IsCheckable<T>,
{
    OneOrMore::new(matcher)
}

impl<T, MContext, Match> HasId for OneOrMore<T, MContext, Match>
where
    Match: HasId,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, MContext, Match> IsCheckable<T> for OneOrMore<T, MContext, Match>
where
    Match: Grammar<T>,
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

impl<T, MContext, Match> Matcher<T, MContext> for OneOrMore<T, MContext, Match>
where
    MContext: Deref<Target = ParserContext<T>>,
    Match: Matcher<T, MContext> + HasId + IsCheckable<T>,
{
    fn match_pattern(&self, context: &mut MContext, pos: &mut usize) -> Result<(), String> {
        // First match is mandatory — propagate the error if absent.
        self.matcher.match_pattern(context, pos)?;
        // Remaining matches are optional (same as Multiple).
        while self.matcher.check_no_advance(context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}
