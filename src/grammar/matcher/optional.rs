use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::ParserContext,
    get_next_id,
    matcher::Matcher,
};
use std::{marker::PhantomData, ops::Deref};

pub struct Optional<T, N, Match> {
    matcher: Match,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Match> Optional<T, N, Match>
where
    Match: Matcher<T, N> + Grammar<T>,
{
    fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

pub fn optional<T, N, Match>(matcher: Match) -> Optional<T, N, Match>
where
    Match: Matcher<T, N> + HasId + IsCheckable<T>,
{
    Optional::new(matcher)
}

impl<T, N, Match> Matcher<T, N> for Optional<T, N, Match>
where
    N: Deref<Target = ParserContext<T>>,
    Match: Matcher<T, N> + Grammar<T>,
{
    fn match_pattern(&self, context: &mut N, pos: &mut usize) -> Result<(), String> {
        if self.matcher.check_no_advance(context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}

impl<T, N, Match> IsCheckable<T> for Optional<T, N, Match>
where
    Match: Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        self.matcher.check(context, pos);
        true
    }
}

impl<T, N, Match> HasId for Optional<T, N, Match> {
    fn id(&self) -> usize {
        self.id
    }
}
