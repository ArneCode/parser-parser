use crate::grammar::{
    AstNode, Grammar, HasId, IsCheckable, Token,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::Matcher,
};
use std::marker::PhantomData;

pub struct Optional<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
{
    matcher: Match,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Match> Optional<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
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
    T: Token + 'static,
    N: AstNode + ?Sized + 'static,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T> + 'static,
{
    Optional::new(matcher)
}

impl<T, N, Match> Matcher<T> for Optional<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + Grammar<T>,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if self.matcher.check_no_advance(context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}

impl<T, N, Match> IsCheckable<T> for Optional<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        self.matcher.check(context, pos);
        true
    }
}

impl<T, N, Match> HasId for Optional<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N>,
{
    fn id(&self) -> usize {
        self.id
    }
}
