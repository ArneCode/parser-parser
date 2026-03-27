use std::{marker::PhantomData, ops::Deref};

use crate::grammar::{
    Grammar, HasId, IsCheckable, context::ParserContext, get_next_id, matcher::Matcher,
};

pub struct Multiple<T, Match> {
    matcher: Match,
    id: usize,
    _phantom: PhantomData<T>,
}

impl<T, Match> Multiple<T, Match> {
    fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

pub fn many<T, Match>(matcher: Match) -> Multiple<T, Match> {
    Multiple::new(matcher)
}
impl<T, MContext, Match> Matcher<T, MContext> for Multiple<T, Match>
where
    MContext: Deref<Target = ParserContext<T>>,
    Match: Matcher<T, MContext> + HasId + IsCheckable<T>,
{
    fn match_pattern(&self, context: &mut MContext, pos: &mut usize) -> Result<(), String> {
        while self.matcher.check_no_advance(context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}

impl<T, Match> IsCheckable<T> for Multiple<T, Match>
where
    Match: Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        while self.matcher.check(context, pos) {}
        true
    }
}

impl<T, Match> HasId for Multiple<T, Match> {
    fn id(&self) -> usize {
        self.id
    }
}
